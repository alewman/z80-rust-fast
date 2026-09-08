//! Conformance kit: run a manifest on this core and emit its trace.
//!
//! Transcribed from `z80_python/conformance.py`. A manifest fully determines
//! a run: memory contents, initial processor state, the host profile, when
//! lifecycle requests arrive, and when to stop. [`ConformanceHost`] is the
//! one host both cores implement, so any difference between the traces is a
//! difference between the CPUs.

use std::io;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::core::{Bus, Fault, Z80};
use crate::state::{CpuState, FIELD_NAMES};
use crate::trace::{step_record, StepRecord};

pub const MANIFEST_SCHEMA_VERSION: u64 = 1;

const BDOS_ENTRY: u16 = 0x0005;
const WARM_BOOT: u16 = 0x0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostProfile {
    /// 64 KiB RAM, every port read returns `port_read_value`, writes discarded.
    Flat,
    /// `Flat` plus the two CP/M traps the ZEX exercisers need.
    CpmMinimal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Nmi,
    Int,
    IntClear,
    Reset,
    ResetClear,
}

/// Bytes to place at `address` before the run starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySegment {
    pub address: u16,
    pub data: Vec<u8>,
}

/// A host lifecycle request applied immediately before boundary `at_step`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub at_step: u64,
    pub kind: EventKind,
    pub vector: u8,
}

/// When the run ends. `max_steps` is mandatory so every run is finite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StopRule {
    pub max_steps: u64,
    pub on_halt: bool,
    pub at_pc: Vec<u16>,
}

/// A complete, deterministic description of one conformance run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub host: HostProfile,
    pub port_read_value: u8,
    pub memory: Vec<MemorySegment>,
    pub initial: CpuState,
    pub events: Vec<Event>,
    pub stop: StopRule,
}

/// The one host every conformance run uses.
pub struct ConformanceHost {
    pub memory: Box<[u8; 0x10000]>,
    pub port_read_value: u8,
    pub output: Vec<u8>,
}

impl ConformanceHost {
    pub fn new(manifest: &Manifest) -> Self {
        let mut memory = Box::new([0u8; 0x10000]);
        for segment in &manifest.memory {
            let start = usize::from(segment.address);
            memory[start..start + segment.data.len()].copy_from_slice(&segment.data);
        }
        ConformanceHost {
            memory,
            port_read_value: manifest.port_read_value,
            output: Vec::new(),
        }
    }
}

// The binaries are separate crates, and without `#[inline]` a non-generic
// function in this crate is a real call from them: every byte the core
// read was a call into read_byte, and every step a call into
// handle_cpm_trap. The attribute is a hint about placement, not a change
// in what the host does.
impl Bus for ConformanceHost {
    #[inline]
    fn read_byte(&mut self, addr: u16) -> u8 {
        self.memory[usize::from(addr)]
    }

    #[inline]
    fn write_byte(&mut self, addr: u16, value: u8) {
        self.memory[usize::from(addr)] = value;
    }

    #[inline]
    fn read_port(&mut self, _addr: u16) -> u8 {
        self.port_read_value
    }

    #[inline]
    fn write_port(&mut self, _addr: u16, _value: u8) {}
}

/// Why a run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StopReason {
    MaxSteps,
    CpmExit,
    AtPc,
    Halted,
    /// The core raised where the reference raises; the trace ends here.
    Fault(Fault),
}

impl StopReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            StopReason::MaxSteps => "max_steps",
            StopReason::CpmExit => "cpm_exit",
            StopReason::AtPc => "at_pc",
            StopReason::Halted => "halted",
            StopReason::Fault(_) => "fault",
        }
    }
}

/// Why a run ended, plus anything the host captured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceRun {
    pub steps: u64,
    pub t_states: u64,
    pub reason: StopReason,
    pub output: Vec<u8>,
}

#[derive(Debug)]
pub enum RunError {
    /// `cpm-minimal` saw a BDOS function it does not support.
    UnsupportedBdosFunction(u8),
    Io(io::Error),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::UnsupportedBdosFunction(function) => {
                write!(
                    f,
                    "cpm-minimal: unsupported BDOS function {function} at PC 0x0005"
                )
            }
            RunError::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for RunError {}

impl From<io::Error> for RunError {
    fn from(error: io::Error) -> Self {
        RunError::Io(error)
    }
}

/// Apply the `cpm-minimal` traps at the current PC; `Ok(true)` means stop.
///
/// Exactly what `ConformanceHost.handle_cpm_trap` does in the reference:
/// PC == 0x0000 ends the run; PC == 0x0005 performs BDOS function C (0 ends
/// the run, 2 appends E, 9 appends bytes from DE up to `$`), then pops the
/// return address into PC.
#[inline]
pub fn handle_cpm_trap(cpu: &mut Z80<ConformanceHost>) -> Result<bool, RunError> {
    if cpu.pc == WARM_BOOT {
        return Ok(true);
    }
    if cpu.pc != BDOS_ENTRY {
        return Ok(false);
    }
    bdos_call(cpu)
}

/// The BDOS function part of [`handle_cpm_trap`], out of line because it
/// runs once per console character, not once per instruction.
#[cold]
#[inline(never)]
fn bdos_call(cpu: &mut Z80<ConformanceHost>) -> Result<bool, RunError> {
    let function = cpu.c;
    if function == 0 {
        return Ok(true);
    }
    if function == 2 {
        cpu.bus.output.push(cpu.e);
    } else if function == 9 {
        let mut address = (u16::from(cpu.d) << 8) | u16::from(cpu.e);
        loop {
            let value = cpu.bus.memory[usize::from(address)];
            address = address.wrapping_add(1);
            if value == b'$' {
                break;
            }
            cpu.bus.output.push(value);
        }
    } else {
        return Err(RunError::UnsupportedBdosFunction(function));
    }
    let low = cpu.bus.memory[usize::from(cpu.sp)];
    let high = cpu.bus.memory[usize::from(cpu.sp.wrapping_add(1))];
    cpu.sp = cpu.sp.wrapping_add(2);
    cpu.pc = (u16::from(high) << 8) | u16::from(low);
    Ok(false)
}

fn apply_event(cpu: &mut Z80<ConformanceHost>, event: Event) {
    match event.kind {
        EventKind::Nmi => cpu.request_non_maskable_interrupt(),
        EventKind::Int => cpu.request_maskable_interrupt(event.vector),
        EventKind::IntClear => cpu.clear_maskable_interrupt(),
        EventKind::Reset => cpu.request_reset(),
        EventKind::ResetClear => cpu.clear_reset(),
    }
}

/// Run `manifest` on this core, handing every record to `sink` in order.
///
/// The stop-check order each boundary is the reference's: pending events,
/// cpm traps, `at_pc`, `on_halt`, then the step budget.
pub fn trace_manifest<F>(manifest: &Manifest, sink: F) -> Result<TraceRun, RunError>
where
    F: FnMut(&StepRecord) -> io::Result<()>,
{
    trace_manifest_with(manifest, sink, |_, _| Ok(()))
}

/// [`trace_manifest`] with a hook called before every boundary that will be
/// recorded, after the stop checks have passed, with the record index and
/// the machine. Used to write checkpoints.
pub fn trace_manifest_with<F, G>(
    manifest: &Manifest,
    mut sink: F,
    mut before_step: G,
) -> Result<TraceRun, RunError>
where
    F: FnMut(&StepRecord) -> io::Result<()>,
    G: FnMut(u64, &Z80<ConformanceHost>) -> io::Result<()>,
{
    let mut cpu = Z80::new(ConformanceHost::new(manifest));
    cpu.restore_state(&manifest.initial);
    let mut events = manifest.events.iter().copied().peekable();
    let stop = &manifest.stop;
    let mut reason = StopReason::MaxSteps;
    let mut steps: u64 = 0;
    let mut t_states: u64 = 0;
    while steps < stop.max_steps {
        while events.peek().is_some_and(|event| event.at_step == steps) {
            apply_event(&mut cpu, events.next().unwrap());
        }
        if manifest.host == HostProfile::CpmMinimal && handle_cpm_trap(&mut cpu)? {
            reason = StopReason::CpmExit;
            break;
        }
        if stop.at_pc.contains(&cpu.pc) {
            reason = StopReason::AtPc;
            break;
        }
        if stop.on_halt
            && cpu.halted
            && events.peek().is_none()
            && !cpu.reset_pending()
            && !cpu.non_maskable_interrupt_pending()
            && !cpu.maskable_interrupt_pending()
        {
            reason = StopReason::Halted;
            break;
        }
        before_step(steps, &cpu)?;
        match step_record(&mut cpu, steps) {
            Ok(record) => {
                sink(&record)?;
                steps += 1;
                t_states += u64::from(record.t_states);
            }
            Err(fault) => {
                reason = StopReason::Fault(fault);
                break;
            }
        }
    }
    Ok(TraceRun {
        steps,
        t_states,
        reason,
        output: std::mem::take(&mut cpu.bus.output),
    })
}

// --- manifest loading ----------------------------------------------------------

fn object<'a>(value: &'a Value, name: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{name} must be an object with string keys"))
}

fn list<'a>(value: &'a Value, name: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| format!("{name} must be a list"))
}

fn allowed(
    value: &serde_json::Map<String, Value>,
    name: &str,
    keys: &[&str],
    required: &[&str],
) -> Result<(), String> {
    let mut unknown: Vec<&str> = value
        .keys()
        .map(String::as_str)
        .filter(|key| !keys.contains(key))
        .collect();
    let mut missing: Vec<&str> = required
        .iter()
        .copied()
        .filter(|key| !value.contains_key(*key))
        .collect();
    if unknown.is_empty() && missing.is_empty() {
        return Ok(());
    }
    unknown.sort_unstable();
    missing.sort_unstable();
    let mut details = Vec::new();
    if !missing.is_empty() {
        details.push(format!("missing={missing:?}"));
    }
    if !unknown.is_empty() {
        details.push(format!("unknown={unknown:?}"));
    }
    Err(format!(
        "{name} fields do not match schema ({})",
        details.join(", ")
    ))
}

fn int_in_range(value: &Value, name: &str, maximum: u64) -> Result<u64, String> {
    match value.as_u64() {
        Some(number) if number <= maximum => Ok(number),
        _ => {
            let width = if maximum == 0xFF { 2 } else { 4 };
            Err(format!(
                "{name} must be an integer in range 0x{:0width$X}..0x{maximum:0width$X}",
                0,
                width = width
            ))
        }
    }
}

fn boolean(value: &Value, name: &str) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("{name} must be a bool"))
}

/// Build a `CpuState` from a JSON object holding any subset of its fields
/// (`partial`), or exactly all 29 of them.
pub fn state_from_json(value: &Value, name: &str, partial: bool) -> Result<CpuState, String> {
    let encoded = object(value, name)?;
    let required: &[&str] = if partial { &[] } else { &FIELD_NAMES };
    allowed(encoded, name, &FIELD_NAMES, required)?;
    let mut state = CpuState::default();
    for (key, item) in encoded {
        let field = format!("{name}.{key}");
        match key.as_str() {
            "a" => state.a = int_in_range(item, &field, 0xFF)? as u8,
            "f" => state.f = int_in_range(item, &field, 0xFF)? as u8,
            "b" => state.b = int_in_range(item, &field, 0xFF)? as u8,
            "c" => state.c = int_in_range(item, &field, 0xFF)? as u8,
            "d" => state.d = int_in_range(item, &field, 0xFF)? as u8,
            "e" => state.e = int_in_range(item, &field, 0xFF)? as u8,
            "h" => state.h = int_in_range(item, &field, 0xFF)? as u8,
            "l" => state.l = int_in_range(item, &field, 0xFF)? as u8,
            "ix" => state.ix = int_in_range(item, &field, 0xFFFF)? as u16,
            "iy" => state.iy = int_in_range(item, &field, 0xFFFF)? as u16,
            "sp" => state.sp = int_in_range(item, &field, 0xFFFF)? as u16,
            "pc" => state.pc = int_in_range(item, &field, 0xFFFF)? as u16,
            "wz" => state.wz = int_in_range(item, &field, 0xFFFF)? as u16,
            "i" => state.i = int_in_range(item, &field, 0xFF)? as u8,
            "r" => state.r = int_in_range(item, &field, 0xFF)? as u8,
            "iff1" => state.iff1 = boolean(item, &field)?,
            "iff2" => state.iff2 = boolean(item, &field)?,
            "im" => {
                state.im = match item.as_u64() {
                    Some(mode @ 0..=2) => mode as u8,
                    _ => return Err("im must be 0, 1, or 2".to_string()),
                }
            }
            "af_alt" => state.af_alt = int_in_range(item, &field, 0xFFFF)? as u16,
            "bc_alt" => state.bc_alt = int_in_range(item, &field, 0xFFFF)? as u16,
            "de_alt" => state.de_alt = int_in_range(item, &field, 0xFFFF)? as u16,
            "hl_alt" => state.hl_alt = int_in_range(item, &field, 0xFFFF)? as u16,
            "q" => state.q = int_in_range(item, &field, 0xFF)? as u8,
            "halted" => state.halted = boolean(item, &field)?,
            "ei_delay" => {
                state.ei_delay = match item.as_u64() {
                    Some(delay @ 0..=1) => delay as u8,
                    _ => return Err("ei_delay must be 0 or 1".to_string()),
                }
            }
            "reset_pending" => state.reset_pending = boolean(item, &field)?,
            "maskable_interrupt_vector" => {
                state.maskable_interrupt_vector = if item.is_null() {
                    None
                } else {
                    Some(int_in_range(item, &field, 0xFF)? as u8)
                }
            }
            "non_maskable_interrupt_pending" => {
                state.non_maskable_interrupt_pending = boolean(item, &field)?
            }
            _ => unreachable!("allowed() rejected unknown keys"),
        }
    }
    Ok(state)
}

fn hex_bytes(text: &str, name: &str) -> Result<Vec<u8>, String> {
    let error = || format!("{name} must be a hexadecimal string");
    if text.len() % 2 != 0 {
        return Err(error());
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).map_err(|_| error()))
        .collect()
}

fn segment(item: &Value, index: usize, base_dir: Option<&Path>) -> Result<MemorySegment, String> {
    let name = format!("memory[{index}]");
    let item = object(item, &name)?;
    allowed(
        item,
        &name,
        &["address", "data", "file", "offset", "length"],
        &["address"],
    )?;
    if item.contains_key("data") == item.contains_key("file") {
        return Err(format!("{name} must have exactly one of 'data' or 'file'"));
    }
    let data = if let Some(data) = item.get("data") {
        let text = data
            .as_str()
            .ok_or_else(|| format!("{name}.data must be a hexadecimal string"))?;
        hex_bytes(text, &format!("{name}.data"))?
    } else {
        let file = item["file"]
            .as_str()
            .ok_or_else(|| format!("{name}.file must be a path string"))?;
        let mut file_path = PathBuf::from(file);
        if !file_path.is_absolute() {
            let base = base_dir
                .map(Path::to_path_buf)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            file_path = base.join(file_path);
        }
        let data = std::fs::read(&file_path)
            .map_err(|error| format!("{name}.file {}: {error}", file_path.display()))?;
        let offset = item.get("offset").map_or(Some(0), Value::as_u64);
        let length = item
            .get("length")
            .map_or(Some(data.len() as u64 - offset.unwrap_or(0)), Value::as_u64);
        match (offset, length) {
            (Some(offset), Some(length)) if length > 0 && offset as usize <= data.len() => {
                let start = offset as usize;
                let end = (start + length as usize).min(data.len());
                data[start..end].to_vec()
            }
            _ => {
                return Err(format!(
                    "{name}.offset/length must be non-negative/positive integers"
                ))
            }
        }
    };
    let address = int_in_range(&item["address"], "segment address", 0xFFFF)?;
    if data.is_empty() {
        return Err(format!("{name}: segment data must be non-empty bytes"));
    }
    if address as usize + data.len() > 0x10000 {
        return Err(format!("{name}: segment does not fit below 0x10000"));
    }
    Ok(MemorySegment {
        address: address as u16,
        data,
    })
}

fn event(item: &Value, index: usize) -> Result<Event, String> {
    let name = format!("events[{index}]");
    let item = object(item, &name)?;
    allowed(
        item,
        &name,
        &["at_step", "kind", "vector"],
        &["at_step", "kind"],
    )?;
    let at_step = item["at_step"]
        .as_u64()
        .ok_or_else(|| format!("{name}: event at_step must be a non-negative integer"))?;
    let kind = match item["kind"].as_str() {
        Some("nmi") => EventKind::Nmi,
        Some("int") => EventKind::Int,
        Some("int_clear") => EventKind::IntClear,
        Some("reset") => EventKind::Reset,
        Some("reset_clear") => EventKind::ResetClear,
        other => {
            return Err(format!(
                "{name}: event kind must be one of ('nmi', 'int', 'int_clear', 'reset', \
                 'reset_clear'), got {other:?}"
            ))
        }
    };
    let vector = match item.get("vector") {
        None => 0xFF,
        Some(vector) => int_in_range(vector, &format!("{name}: event vector"), 0xFF)? as u8,
    };
    Ok(Event {
        at_step,
        kind,
        vector,
    })
}

/// Build a validated manifest from its JSON form.
pub fn manifest_from_json(value: &Value, base_dir: Option<&Path>) -> Result<Manifest, String> {
    let root = object(value, "manifest")?;
    allowed(
        root,
        "manifest",
        &[
            "version",
            "name",
            "host",
            "port_read_value",
            "memory",
            "initial",
            "events",
            "stop",
        ],
        &["version", "name", "memory", "stop"],
    )?;
    if root["version"].as_u64() != Some(MANIFEST_SCHEMA_VERSION) {
        return Err(format!("unsupported manifest version: {}", root["version"]));
    }
    let name = match root["name"].as_str() {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => return Err("manifest name must be a non-empty string".to_string()),
    };
    let memory = list(&root["memory"], "memory")?
        .iter()
        .enumerate()
        .map(|(index, item)| segment(item, index, base_dir))
        .collect::<Result<Vec<_>, _>>()?;
    let empty = Value::Object(serde_json::Map::new());
    let initial = state_from_json(root.get("initial").unwrap_or(&empty), "initial", true)
        .map_err(|error| format!("invalid initial state: {error}"))?;
    let stop_dict = object(&root["stop"], "stop")?;
    allowed(
        stop_dict,
        "stop",
        &["max_steps", "on_halt", "at_pc"],
        &["max_steps"],
    )?;
    let max_steps = match stop_dict["max_steps"].as_u64() {
        Some(steps) if steps > 0 => steps,
        _ => return Err("stop.max_steps must be a positive integer".to_string()),
    };
    let on_halt = match stop_dict.get("on_halt") {
        None => true,
        Some(value) => boolean(value, "stop.on_halt")?,
    };
    let at_pc = match stop_dict.get("at_pc") {
        None => Vec::new(),
        Some(value) => list(value, "stop.at_pc")?
            .iter()
            .map(|pc| {
                pc.as_u64()
                    .filter(|pc| *pc <= 0xFFFF)
                    .map(|pc| pc as u16)
                    .ok_or_else(|| "stop.at_pc must be a tuple of 16-bit addresses".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?,
    };
    let events = match root.get("events") {
        None => Vec::new(),
        Some(value) => list(value, "events")?
            .iter()
            .enumerate()
            .map(|(index, item)| event(item, index))
            .collect::<Result<Vec<_>, _>>()?,
    };
    if events
        .windows(2)
        .any(|pair| pair[0].at_step > pair[1].at_step)
    {
        return Err("manifest events must be ordered by at_step".to_string());
    }
    let host = match root.get("host").map(|host| host.as_str()) {
        None | Some(Some("flat")) => HostProfile::Flat,
        Some(Some("cpm-minimal")) => HostProfile::CpmMinimal,
        Some(other) => {
            return Err(format!(
                "manifest host must be one of ('flat', 'cpm-minimal'), got {other:?}"
            ))
        }
    };
    let port_read_value = match root.get("port_read_value") {
        None => 0xFF,
        Some(value) => int_in_range(value, "manifest port_read_value", 0xFF)? as u8,
    };
    Ok(Manifest {
        name,
        host,
        port_read_value,
        memory,
        initial,
        events,
        stop: StopRule {
            max_steps,
            on_halt,
            at_pc,
        },
    })
}

/// Read and validate a manifest file; `file` segments resolve beside it.
pub fn load_manifest(path: &Path) -> Result<Manifest, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{}: not valid JSON: {error}", path.display()))?;
    manifest_from_json(&value, path.parent())
}

// --- checkpoints ---------------------------------------------------------------

/// Write a manifest that resumes this run from the machine's current state.
///
/// The checkpoint carries the full 64 KiB as a `file` segment beside the
/// manifest, every `CpuState` field as `initial`, and `max_steps` of
/// `segment_steps`, so running the checkpoints of a long trace in parallel
/// and diffing each one against the reference proves the same thing the
/// single lockstep run proves: each segment starts from the state the
/// previous segment ended in, so a divergence anywhere is reported by the
/// segment that contains it. Manifests with `events` are refused, because
/// their `at_step` values would have to be shifted.
pub fn write_checkpoint(
    manifest: &Manifest,
    cpu: &Z80<ConformanceHost>,
    at_step: u64,
    segment_steps: u64,
    dir: &Path,
) -> io::Result<PathBuf> {
    if !manifest.events.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "checkpoints are not supported for manifests with events",
        ));
    }
    let stem = format!("{}-{:012}", manifest.name, at_step);
    let memory_name = format!("{stem}.mem");
    std::fs::write(dir.join(&memory_name), &cpu.bus.memory[..])?;
    let state = cpu.capture_state();
    let mut initial = String::new();
    state.write_json(&mut initial);
    let initial: Value = serde_json::from_str(&initial).expect("state JSON is valid");
    let document = serde_json::json!({
        "version": MANIFEST_SCHEMA_VERSION,
        "name": stem,
        "host": match manifest.host {
            HostProfile::Flat => "flat",
            HostProfile::CpmMinimal => "cpm-minimal",
        },
        "port_read_value": manifest.port_read_value,
        "memory": [{"address": 0, "file": memory_name}],
        "initial": initial,
        "events": [],
        "stop": {
            "max_steps": segment_steps,
            "on_halt": manifest.stop.on_halt,
            "at_pc": manifest.stop.at_pc,
        },
    });
    let path = dir.join(format!("{stem}.json"));
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&document)?),
    )?;
    Ok(path)
}

//! In-process PipeWire virtual cables.
//!
//! A virtual input is a null sink. Applications play into it and a capture
//! stream reads its monitor. A virtual output is a virtual source. This client
//! links a playback stream to the source's input ports; applications record
//! the source. `object.linger` is false, so the nodes disappear with the client.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use audio_program_engine::{Frame, Shared, MAX_CHANNELS};
use pipewire as pw;
use pw::properties::properties;
use pw::spa;
use pw::spa::pod::Pod;

use crate::{BackendError, CableSpec, HardwareDevice, VirtualCable, VirtualCableBackend};

const CHANNELS: usize = 2;

struct PortRec {
    global_id: u32,
    node_id: u32,
    port_id: u32,
    direction: String,
    channel: String,
}

struct NodeRec {
    name: String,
    class: String,
}

struct StreamData {
    shared: Arc<Shared>,
    index: usize,
    playback: bool,
}

struct Cable {
    node_name: String,
    output: bool,
    stream: pw::stream::StreamRc,
    _listener: pw::stream::StreamListener<StreamData>,
    reply: Option<Sender<Result<(), String>>>,
    started: Instant,
    /// Bit 0 is FL, bit 1 is FR. Capture cables set both when the stream has a node.
    linked_mask: u8,
}

struct Inner {
    nodes: Vec<pw::node::Node>,
    cables: Vec<Cable>,
    links: Vec<pw::link::Link>,
    ports: Vec<PortRec>,
    node_info: Vec<(u32, NodeRec)>,
}

enum Command {
    Create {
        spec: CableSpec,
        shared: Arc<Shared>,
        output: bool,
        reply: Sender<Result<(), String>>,
    },
    List(Sender<Vec<HardwareDevice>>),
    Shutdown,
}

pub struct PipeWireBackend {
    commands: pw::channel::Sender<Command>,
    thread: Option<JoinHandle<()>>,
}

impl PipeWireBackend {
    pub fn connect() -> Result<Self, BackendError> {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (command_tx, command_rx) = pw::channel::channel();
        let thread = std::thread::Builder::new()
            .name("audio-program-pipewire".to_string())
            .spawn(move || pipewire_thread(command_rx, ready_tx))
            .map_err(|err| BackendError::Unavailable(err.to_string()))?;
        match ready_rx.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(())) => Ok(Self {
                commands: command_tx,
                thread: Some(thread),
            }),
            Ok(Err(message)) => Err(BackendError::Unavailable(message)),
            Err(_) => Err(BackendError::Unavailable(
                "PipeWire did not answer. Is the daemon running?".to_string(),
            )),
        }
    }
}

impl Drop for PipeWireBackend {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl VirtualCableBackend for PipeWireBackend {
    fn list_hardware(&self) -> Result<Vec<HardwareDevice>, BackendError> {
        let (tx, rx) = mpsc::channel();
        self.commands
            .send(Command::List(tx))
            .map_err(|_| BackendError::Unavailable("PipeWire thread stopped".to_string()))?;
        rx.recv_timeout(Duration::from_secs(2))
            .map_err(|_| BackendError::Unavailable("PipeWire did not list devices".to_string()))
    }

    fn create_virtual_input(
        &mut self,
        spec: CableSpec,
        shared: &Arc<Shared>,
    ) -> Result<VirtualCable, BackendError> {
        self.create(spec, shared, false)
    }

    fn create_virtual_output(
        &mut self,
        spec: CableSpec,
        shared: &Arc<Shared>,
    ) -> Result<VirtualCable, BackendError> {
        self.create(spec, shared, true)
    }
}

impl PipeWireBackend {
    fn create(
        &self,
        spec: CableSpec,
        shared: &Arc<Shared>,
        output: bool,
    ) -> Result<VirtualCable, BackendError> {
        let cable = VirtualCable {
            node_name: spec.node_name.clone(),
            description: spec.description.clone(),
            index: spec.index,
            output,
        };
        let (tx, rx) = mpsc::channel();
        self.commands
            .send(Command::Create {
                spec,
                shared: Arc::clone(shared),
                output,
                reply: tx,
            })
            .map_err(|_| BackendError::Unavailable("PipeWire thread stopped".to_string()))?;
        match rx.recv_timeout(Duration::from_secs(4)) {
            Ok(Ok(())) => Ok(cable),
            Ok(Err(message)) => Err(BackendError::Unavailable(message)),
            Err(_) => Err(BackendError::Unavailable(
                "timed out creating a PipeWire virtual cable".to_string(),
            )),
        }
    }
}

fn pipewire_thread(commands: pw::channel::Receiver<Command>, ready: Sender<Result<(), String>>) {
    pw::init();
    let mainloop = match pw::main_loop::MainLoopRc::new(None) {
        Ok(mainloop) => mainloop,
        Err(err) => {
            let _ = ready.send(Err(err.to_string()));
            return;
        }
    };
    let context = match pw::context::ContextRc::new(&mainloop, None) {
        Ok(context) => context,
        Err(err) => {
            let _ = ready.send(Err(err.to_string()));
            return;
        }
    };
    let core = match context.connect_rc(None) {
        Ok(core) => core,
        Err(err) => {
            let _ = ready.send(Err(format!(
                "could not connect to PipeWire ({err}). Start the pipewire daemon and WirePlumber."
            )));
            return;
        }
    };

    let inner = Rc::new(RefCell::new(Inner {
        nodes: Vec::new(),
        cables: Vec::new(),
        links: Vec::new(),
        ports: Vec::new(),
        node_info: Vec::new(),
    }));

    let registry = match core.get_registry() {
        Ok(registry) => registry,
        Err(err) => {
            let _ = ready.send(Err(err.to_string()));
            return;
        }
    };
    let listed = inner.clone();
    let _registry_listener = registry
        .add_listener_local()
        .global(move |global| remember_global(&listed, global))
        .register();

    let _ = ready.send(Ok(()));

    let loop_for_commands = mainloop.clone();
    let core_for_commands = core.clone();
    let inner_for_commands = inner.clone();
    let _commands = commands.attach(mainloop.loop_(), move |command| match command {
        Command::Shutdown => loop_for_commands.quit(),
        Command::List(reply) => {
            let devices = hardware_from(&inner_for_commands.borrow());
            let _ = reply.send(devices);
        }
        Command::Create {
            spec,
            shared,
            output,
            reply,
        } => {
            if let Err(err) = spawn_cable(
                &core_for_commands,
                &inner_for_commands,
                spec,
                shared,
                output,
                reply.clone(),
            ) {
                let _ = reply.send(Err(err));
            }
        }
    });

    let core_for_timer = core.clone();
    let inner_for_timer = inner.clone();
    let _timer = mainloop.loop_().add_timer(move |_| {
        finish_cables(&core_for_timer, &inner_for_timer);
    });
    _timer.update_timer(
        Some(Duration::from_millis(50)),
        Some(Duration::from_millis(50)),
    );

    mainloop.run();
}

fn remember_global(
    inner: &Rc<RefCell<Inner>>,
    global: &pw::registry::GlobalObject<&spa::utils::dict::DictRef>,
) {
    let Some(props) = global.props else {
        return;
    };
    let mut inner = inner.borrow_mut();
    if global.type_ == pw::types::ObjectType::Node {
        if let Some(name) = props.get("node.name") {
            if inner.node_info.iter().any(|(id, _)| *id == global.id) {
                return;
            }
            inner.node_info.push((
                global.id,
                NodeRec {
                    name: name.to_string(),
                    class: props.get("media.class").unwrap_or("").to_string(),
                },
            ));
        }
        return;
    }
    if global.type_ != pw::types::ObjectType::Port {
        return;
    }
    if inner.ports.iter().any(|port| port.global_id == global.id) {
        return;
    }
    let Some(node_id) = props.get("node.id").and_then(|value| value.parse().ok()) else {
        return;
    };
    let Some(port_id) = props.get("port.id").and_then(|value| value.parse().ok()) else {
        return;
    };
    inner.ports.push(PortRec {
        global_id: global.id,
        node_id,
        port_id,
        direction: props.get("port.direction").unwrap_or("").to_string(),
        channel: props.get("audio.channel").unwrap_or("").to_string(),
    });
}

fn hardware_from(inner: &Inner) -> Vec<HardwareDevice> {
    inner
        .node_info
        .iter()
        .filter_map(|(id, info)| {
            hardware_output(&info.name, &info.class).map(|output| HardwareDevice {
                id: id.to_string(),
                name: info.name.clone(),
                output,
            })
        })
        .collect()
}

/// `Some(true)` is a playback device. Our own cables and virtual sources are skipped.
fn hardware_output(name: &str, class: &str) -> Option<bool> {
    if name.starts_with("audio-program-") {
        return None;
    }
    match class {
        "Audio/Sink" => Some(true),
        "Audio/Source" => Some(false),
        _ => None,
    }
}

/// How many frames to write into a playback buffer.
///
/// PipeWire's mapped buffer can be longer than the audio it asked for, and
/// longer than the mixer ring. Filling the spare space with silence is a gap
/// in the recording, so this stops at the frames the mixer actually has.
fn playback_frame_count(
    chunk_bytes: usize,
    buffer_bytes: usize,
    stride: usize,
    available: usize,
) -> usize {
    if stride == 0 || buffer_bytes < stride {
        return 0;
    }
    let requested = if chunk_bytes >= stride {
        chunk_bytes / stride
    } else {
        buffer_bytes / stride
    };
    requested.min(available).min(buffer_bytes / stride)
}

fn spawn_cable(
    core: &pw::core::CoreRc,
    inner: &Rc<RefCell<Inner>>,
    spec: CableSpec,
    shared: Arc<Shared>,
    output: bool,
    reply: Sender<Result<(), String>>,
) -> Result<(), String> {
    let media_class = if output {
        "Audio/Source/Virtual"
    } else {
        "Audio/Sink"
    };
    let node_props = properties! {
        "factory.name" => "support.null-audio-sink",
        "node.name" => spec.node_name.clone(),
        "node.description" => spec.description.clone(),
        "media.class" => media_class,
        "audio.position" => "FL,FR",
        "object.linger" => "false",
    };
    let node = core
        .create_object::<pw::node::Node>("adapter", &node_props)
        .map_err(|err| format!("could not create {}: {err}", spec.node_name))?;

    let stream_name = format!("{}-stream", spec.node_name);
    let stream_props = if output {
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Playback",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::NODE_NAME => stream_name.clone(),
            "node.autoconnect" => "false",
            "node.dont-reconnect" => "true",
            "node.passive" => "true",
            *pw::keys::NODE_LATENCY => "480/48000",
        }
    } else {
        properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Music",
            *pw::keys::NODE_NAME => stream_name.clone(),
            *pw::keys::STREAM_CAPTURE_SINK => "true",
            *pw::keys::TARGET_OBJECT => spec.node_name.clone(),
            *pw::keys::NODE_LATENCY => "480/48000",
        }
    };
    let stream = pw::stream::StreamRc::new(core.clone(), &stream_name, stream_props)
        .map_err(|err| err.to_string())?;
    let data = StreamData {
        shared,
        index: spec.index,
        playback: output,
    };
    let listener = stream
        .add_local_listener_with_user_data(data)
        .process(copy_stream)
        .register()
        .map_err(|err| err.to_string())?;

    let format = audio_format_pod();
    let pod = Pod::from_bytes(&format).ok_or("audio format pod")?;
    let mut params = [pod];
    let flags = if output {
        pw::stream::StreamFlags::MAP_BUFFERS | pw::stream::StreamFlags::RT_PROCESS
    } else {
        pw::stream::StreamFlags::AUTOCONNECT
            | pw::stream::StreamFlags::MAP_BUFFERS
            | pw::stream::StreamFlags::RT_PROCESS
    };
    let direction = if output {
        spa::utils::Direction::Output
    } else {
        spa::utils::Direction::Input
    };
    stream
        .connect(direction, None, flags, &mut params)
        .map_err(|err| err.to_string())?;

    let mut inner = inner.borrow_mut();
    inner.nodes.push(node);
    inner.cables.push(Cable {
        node_name: spec.node_name,
        output,
        stream,
        _listener: listener,
        reply: Some(reply),
        started: Instant::now(),
        linked_mask: 0,
    });
    Ok(())
}

fn finish_cables(core: &pw::core::CoreRc, inner: &Rc<RefCell<Inner>>) {
    let mut inner = inner.borrow_mut();
    let port_snapshot: Vec<(u32, u32, String, String)> = inner
        .ports
        .iter()
        .map(|port| {
            (
                port.node_id,
                port.port_id,
                port.direction.clone(),
                port.channel.clone(),
            )
        })
        .collect();
    let names: Vec<(u32, String)> = inner
        .node_info
        .iter()
        .map(|(id, info)| (*id, info.name.clone()))
        .collect();

    let mut created_links = Vec::new();
    for cable in &mut inner.cables {
        if cable.linked_mask == 0b11 {
            continue;
        }
        if cable.started.elapsed() > Duration::from_secs(3) {
            if let Some(reply) = cable.reply.take() {
                let _ = reply.send(Err(format!(
                    "PipeWire did not expose ports for {}. Is WirePlumber running?",
                    cable.node_name
                )));
            }
            cable.linked_mask = 0b11;
            continue;
        }
        let stream_node = cable.stream.node_id();
        if stream_node == u32::MAX {
            continue;
        }
        if !cable.output {
            cable.linked_mask = 0b11;
            if let Some(reply) = cable.reply.take() {
                let _ = reply.send(Ok(()));
            }
            continue;
        }
        let Some(source_node) = names
            .iter()
            .find(|(_, name)| name == &cable.node_name)
            .map(|(id, _)| *id)
        else {
            continue;
        };
        for (bit, channel) in [(1u8, "FL"), (2u8, "FR")] {
            if cable.linked_mask & bit != 0 {
                continue;
            }
            let output_port = port_snapshot
                .iter()
                .find(|port| port.0 == stream_node && port.2 == "out" && port.3 == channel);
            let input_port = port_snapshot
                .iter()
                .find(|port| port.0 == source_node && port.2 == "in" && port.3 == channel);
            let (Some(output_port), Some(input_port)) = (output_port, input_port) else {
                continue;
            };
            let props = properties! {
                "link.output.node" => stream_node.to_string(),
                "link.output.port" => output_port.1.to_string(),
                "link.input.node" => source_node.to_string(),
                "link.input.port" => input_port.1.to_string(),
                "object.linger" => "false",
            };
            match core.create_object::<pw::link::Link>("link-factory", &props) {
                Ok(link) => {
                    created_links.push(link);
                    cable.linked_mask |= bit;
                }
                Err(err) => eprintln!("link {channel} for {}: {err}", cable.node_name),
            }
        }
        if cable.linked_mask == 0b11 {
            if let Some(reply) = cable.reply.take() {
                let _ = reply.send(Ok(()));
            }
        }
    }
    inner.links.extend(created_links);
}

fn copy_stream(stream: &pw::stream::Stream, data: &mut StreamData) {
    let Some(mut buffer) = stream.dequeue_buffer() else {
        return;
    };
    let datas = buffer.datas_mut();
    if datas.is_empty() {
        return;
    }
    let stride = CHANNELS * std::mem::size_of::<f32>();
    // `data()` is the whole mapped buffer. The valid capture, and the playback
    // duration PipeWire asked for, are the chunk size. Reading or filling
    // `maxsize` invents a long run of silence around a few real frames.
    let chunk_offset = datas[0].chunk().offset() as usize;
    let chunk_size = datas[0].chunk().size() as usize;
    let frames = {
        let Some(bytes) = datas[0].data() else {
            return;
        };
        if data.playback {
            let frames = playback_frame_count(
                chunk_size,
                bytes.len(),
                stride,
                data.shared.output_available(data.index),
            );
            let mut frame: Frame;
            for index in 0..frames {
                frame = data
                    .shared
                    .pop_output_frame(data.index)
                    .unwrap_or([0.0; MAX_CHANNELS]);
                let start = index * stride;
                bytes[start..start + 4].copy_from_slice(&frame[0].to_le_bytes());
                bytes[start + 4..start + 8].copy_from_slice(&frame[1].to_le_bytes());
            }
            frames
        } else {
            let start = chunk_offset.min(bytes.len());
            let end = start.saturating_add(chunk_size).min(bytes.len());
            let valid = &bytes[start..end];
            let frames = valid.len() / stride;
            let mut frame: Frame = [0.0; MAX_CHANNELS];
            for index in 0..frames {
                let at = index * stride;
                frame[0] = f32::from_le_bytes(valid[at..at + 4].try_into().unwrap_or([0; 4]));
                frame[1] = f32::from_le_bytes(valid[at + 4..at + 8].try_into().unwrap_or([0; 4]));
                data.shared.push_input_frame(data.index, &frame);
            }
            frames
        }
    };
    if data.playback {
        let chunk = datas[0].chunk_mut();
        *chunk.offset_mut() = 0;
        *chunk.stride_mut() = stride as i32;
        *chunk.size_mut() = (stride * frames) as u32;
    }
}

fn audio_format_pod() -> Vec<u8> {
    let mut info = spa::param::audio::AudioInfoRaw::new();
    info.set_format(spa::param::audio::AudioFormat::F32LE);
    info.set_rate(48_000);
    info.set_channels(CHANNELS as u32);
    let mut position = [0; spa::param::audio::MAX_CHANNELS];
    position[0] = spa::sys::SPA_AUDIO_CHANNEL_FL;
    position[1] = spa::sys::SPA_AUDIO_CHANNEL_FR;
    info.set_position(position);
    let object = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: info.into(),
    };
    spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(object),
    )
    .map(|pod| pod.0.into_inner())
    .unwrap_or_default()
}

#[cfg(test)]
#[path = "pipewire_tests.rs"]
mod tests;

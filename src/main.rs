use wgpu::{Adapter, Buffer, Device, Queue, util::DeviceExt};

// *** If this value is false, then trying to use GPU captures in Xcode will fail, as it does
// not see the data the buffer was created with when using `create_buffer_init`, just zeroes ***
const USE_MAPPABLE_BUFFERS: bool = false;

const BUFFER_ENTRIES: u64 = 64;
const BUFFER_SIZE: u64 = BUFFER_ENTRIES * (std::mem::size_of::<u32>() as u64);
const WORKGROUP_SIZE: u32 = 8;

fn main() {
    futures::executor::block_on(run());
}

fn get_data_buffer(device: &Device) -> Buffer {
    let buffer_contents: Vec<u32> = (100..(100 + BUFFER_ENTRIES as u32)).collect();

    if USE_MAPPABLE_BUFFERS {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dbg data buffer"),
            size: BUFFER_SIZE,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });

        buffer
            .slice(..)
            .get_mapped_range_mut()
            .copy_from_slice(bytemuck::cast_slice(&buffer_contents));

        buffer.unmap();
        buffer
    } else {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dbg data buffer"),
            contents: bytemuck::cast_slice(&buffer_contents),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        })
    }
}

async fn run() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter: Adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Failed to create an adapter");

    let (device, queue): (Device, Queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: if USE_MAPPABLE_BUFFERS {
                wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
            } else {
                wgpu::Features::empty()
            },
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .expect("failed to create device");

    unsafe {
        device.start_graphics_debugger_capture();
    }

    let shader_module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

    let data_buffer = get_data_buffer(&device);

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Dbg download buffer"),
        size: BUFFER_SIZE,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Dbg bind group layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Dbg bind group"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: data_buffer.as_entire_binding(),
        }],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Dbg pipeline"),
        layout: Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Dbg pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            }),
        ),
        module: &shader_module,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions {
            ..Default::default()
        },
        cache: None,
    });

    let mut compute_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Dbg Command Encoder"),
    });

    let mut compute_pass = compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("Dbg Compute Pass"),
        timestamp_writes: None,
    });

    compute_pass.set_pipeline(&pipeline);

    compute_pass.set_bind_group(0, &bind_group, &[]);
    compute_pass.dispatch_workgroups(BUFFER_ENTRIES as u32 / WORKGROUP_SIZE, 1, 1);

    drop(compute_pass);

    compute_encoder.copy_buffer_to_buffer(
        &data_buffer,
        0,
        &download_buffer,
        0,
        download_buffer.size(),
    );

    let command_buffer = compute_encoder.finish();
    queue.submit([command_buffer]);

    // Wait for the GPU to finish processing the commands
    let (sender, receiver) = futures::channel::oneshot::channel();

    let buffer_slice = download_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, move |_| {
        sender.send(()).unwrap();
    });

    // On native, drive the GPU and mapping to completion. No-op on the web (where it automatically polls).
    device.poll(wgpu::PollType::Wait).unwrap();
    receiver.await.expect("Failed to receive map completion");

    let data = buffer_slice.get_mapped_range();
    let results: Vec<u32> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    download_buffer.unmap();

    unsafe {
        device.stop_graphics_debugger_capture();
    }

    println!("Results: {:?}", results);
}

// Uncomment these following global attributes to silence most warnings of "low" interest:
/*
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
*/
extern crate nalgebra_glm as glm;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::{mem, os::raw::c_void, ptr};

mod shader;
mod util;
mod mesh;
mod scene_graph;
use scene_graph::SceneNode;

use glm::{Mat3x3, Mat4x4, pi};
use glutin::event::{
    DeviceEvent,
    ElementState::{Pressed, Released},
    Event, KeyboardInput,
    VirtualKeyCode::{self, *},
    WindowEvent,
};
use glutin::event_loop::ControlFlow;

// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  byte_size_of_array(my_array)
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
// ptr::null()

// == // Generate your VAO here
unsafe fn create_vao(vertices: &Vec<f32>, indices: &Vec<u32>, normals: &Vec<f32>) -> u32 {
    // Implemented: Generated vao
    let mut vao = 0;

    gl::GenVertexArrays(1, &mut vao);
    gl::BindVertexArray(vao);

    // Vertex buffer
    let mut vbo = 0;
    gl::GenBuffers(1, &mut vbo);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(vertices),
        vertices.as_ptr() as *const c_void, // c_void just means pointer to generic data
        gl::STATIC_DRAW,
    );

    gl::EnableVertexAttribArray(0);

    // Position: x, y, z, 1
    gl::VertexAttribPointer(
        0,
        3,
        gl::FLOAT,
        gl::FALSE,
        3 * size_of::<f32>(), // 8 because each vertex contains x,y,x,1,r,g,b,alpha attributes and distanse to next is 12 bytes
        ptr::null(),
    );

    // Normals buffer

    let mut normals_vao = 0;
    gl::GenBuffers(1, &mut normals_vao);
    gl::BindBuffer(gl::ARRAY_BUFFER, normals_vao);

    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(normals),
        normals.as_ptr() as *const c_void, // c_void just means pointer to generic data
        gl::STATIC_DRAW,
    );

    gl::EnableVertexAttribArray(1);

    gl::VertexAttribPointer(
        1,
        3,
        gl::FLOAT,
        gl::FALSE,
        3 * size_of::<f32>(), // 8 because each vertex contains x,y,x,1,r,g,b,alpha attributes and distanse to next is 12 bytes
        ptr::null(),
    );

    // Index buffer
    let mut index_buffer = 0;
    gl::GenBuffers(1, &mut index_buffer); // count=1 bc we only generate 1 index
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, index_buffer);

    gl::BufferData(
        gl::ELEMENT_ARRAY_BUFFER,
        byte_size_of_array(indices),
        indices.as_ptr() as *const c_void,
        gl::STATIC_DRAW,
    );

    vao
}

    unsafe fn draw_scene(node: &scene_graph::SceneNode,
        view_projection_matrix: &glm::Mat4,
        transformation_so_far: &glm::Mat4,
        shader: &shader::Shader)
    {
        let node_rotation_matrix = glm::rotation(node.rotation.x, &glm::vec3(1.0, 0.0, 0.0))*glm::rotation(node.rotation.y, &glm::vec3(0.0, 1.0, 0.0))*glm::rotation(node.rotation.z, &glm::vec3(0.0, 0.0, 1.0));
        let node_translation_matrix = glm::translation(&node.reference_point);
        let mut perm_transformation_so_far = transformation_so_far*node_translation_matrix;
        let mut perm_view_projection_matrix = view_projection_matrix*transformation_so_far*node_rotation_matrix;
        // logic before drawing the node

        // check if node drawable, set uniforms, bind vao, draw vao
        if node.index_count != -1 {
            node.print();
            let loc = shader.get_uniform_location("camera_transformation_matrix");
            gl::UniformMatrix4fv(loc, 1, gl::FALSE, perm_view_projection_matrix.as_ptr());

            gl::BindVertexArray(node.vao_id);

            gl::DrawElements(
                gl::TRIANGLES,
                node.index_count as i32,
                gl::UNSIGNED_INT,
                ptr::null(),
            );
        }

        // Recurse
        for child in &node.children {
            draw_scene(&**child, &perm_view_projection_matrix, &perm_transformation_so_far, &shader);
        }
    }

fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize::new(
            INITIAL_SCREEN_W,
            INITIAL_SCREEN_H,
        ));
    let cb = glutin::ContextBuilder::new().with_vsync(true);
    let windowed_context = cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    // windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    // windowed_context.window().set_cursor_visible(false);

    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Set up shared tuple for tracking changes to the window size
    let arc_window_size = Arc::new(Mutex::new((INITIAL_SCREEN_W, INITIAL_SCREEN_H, false)));
    // Make a reference of this tuple to send to the render thread
    let window_size = Arc::clone(&arc_window_size);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers.
        // This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        let mut window_aspect_ratio = INITIAL_SCREEN_W as f32 / INITIAL_SCREEN_H as f32;

        // Initializing the different transformation matrixes
        // let scaling_matrix: Mat4x4 =
        //     glm::Mat4::new(
        //         -1.0, 0.0, 0.0, 0.0,
        //         0.0, -1.0, 0.0, 0.0,
        //         0.0, 0.0, -1.0, 0.0,
        //         0.0, 0.0, 0.0, 1.0,
        //     );

        // let translation_matrix: Mat4x4 =
        //     glm::Mat4::new(
        //         1.0, 0.0, 0.0, 0.0, 
        //         0.0, 1.0, 0.0, 0.0,
        //         0.0, 0.0, 1.0, 0.0,
        //         0.0, 0.0, 1.0, 1.0,
        //     );
        // let translation_matrix: Mat4x4 =
        //     glm::translation(&glm::vec3(0.0, 0.0, -3.0));

        // let projection_matrix: glm::Mat4 =
        //     glm::perspective(
        //         window_aspect_ratio,
        //         (std::f32::consts::PI)/2.0,
        //         1.0,
        //         100.0,
        //     );

        // // The final transformation matrix is a combination of all the previously set transformation matrix
        // Projection needs to be applied last, so it is the first matrix we multiply
        // let transformation_matrix = projection_matrix * translation_matrix;

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!(
                "{}: {}",
                util::get_gl_string(gl::VENDOR),
                util::get_gl_string(gl::RENDERER)
            );
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!(
                "GLSL\t: {}",
                util::get_gl_string(gl::SHADING_LANGUAGE_VERSION)
            );
        }

        // == // Set up your VAO around here
        // Assignment 3 Task 1 here
        let terrain_mesh = mesh::Terrain::load("resources/lunarsurface.obj");
        let vao_terrain = unsafe { create_vao(&terrain_mesh.vertices, &terrain_mesh.indices, &terrain_mesh.normals)};

        // Assignment 3 Task 2 here
        let helicopter_mesh = mesh::Helicopter::load("resources/helicopter.obj");
        let vao_body = unsafe { create_vao(&helicopter_mesh.body.vertices, &helicopter_mesh.body.indices, &helicopter_mesh.body.normals)};
        let vao_door = unsafe { create_vao(&helicopter_mesh.door.vertices, &helicopter_mesh.door.indices, &helicopter_mesh.door.normals)};
        let vao_main_rotor = unsafe { create_vao(&helicopter_mesh.main_rotor.vertices, &helicopter_mesh.main_rotor.indices, &helicopter_mesh.main_rotor.normals)};
        let vao_tail_rotor = unsafe { create_vao(&helicopter_mesh.tail_rotor.vertices, &helicopter_mesh.tail_rotor.indices, &helicopter_mesh.tail_rotor.normals)};

        // Scene graph for Assignment 3 Task 2
        // let mut scene_graph = scene_graph::SceneNode::from_vao(vao_terrain, terrain_mesh.indices.len() as i32);
        let mut terrain_node = scene_graph::SceneNode::from_vao(vao_terrain, terrain_mesh.indices.len() as i32);
        let mut body_node = scene_graph::SceneNode::from_vao(vao_body, helicopter_mesh.body.indices.len() as i32);

        let mut door_node = scene_graph::SceneNode::from_vao(vao_door, helicopter_mesh.door.indices.len() as i32);

        let mut main_rotor_node = scene_graph::SceneNode::from_vao(vao_main_rotor, helicopter_mesh.main_rotor.indices.len() as i32);

        let mut tail_rotor_node = scene_graph::SceneNode::from_vao(vao_tail_rotor, helicopter_mesh.tail_rotor.indices.len() as i32);
        // scene_graph.add_child(&helicopter_node);
        let mut scene_graph = scene_graph::SceneNode::new();

        body_node.add_child(&door_node);
        body_node.add_child(&main_rotor_node);
        body_node.add_child(&tail_rotor_node);
        terrain_node.add_child(&body_node);
        scene_graph.add_child(&terrain_node);

        // Setting the initial positions and rotations of every helicopter node
        body_node.position = glm::vec3(0.0, 10.0, 0.0);
        door_node.position = glm::vec3(0.0, 0.0, 0.0);
        main_rotor_node.position = glm::vec3(0.0, 0.0, 0.0);
        tail_rotor_node.position = glm::vec3(0.0, 0.0, 0.0);


        body_node.rotation = glm::vec3(0.0, 0.0, 0.0);
        door_node.rotation = glm::vec3(0.0, 0.0, 0.0);
        main_rotor_node.rotation = glm::vec3(1.5, 1.5, 1.5);
        tail_rotor_node.rotation = glm::vec3(0.0, 0.0, 0.0);

        // Setting the reference points for every helicopter node
        door_node.reference_point = glm::vec3(0.0, 1.5, 10.4);
        door_node.reference_point = glm::vec3(1.0, 0.0, 10.4);
        main_rotor_node.reference_point = glm::vec3(0.0, 2.3, 10.4);
        tail_rotor_node.reference_point = glm::vec3(0.35, 2.3, 10.4);

        // == // Set up your shaders here

        // Basic usage of shader helper:
        // The example code below creates a 'shader' object.
        // It which contains the field `.program_id` and the method `.activate()`.
        // The `.` in the path is relative to `Cargo.toml`.
        // This snippet is not enough to do the exercise, and will need to be modified (outside
        // of just using the correct path), but it only needs to be called once

        let simple_shader = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("./shaders/simple.vert")
                .attach_file("./shaders/simple.frag")
                .link()
        };

        unsafe {
            simple_shader.activate();
        }

        // Used to demonstrate keyboard handling for exercise 2.
        let mut _arbitrary_number = 0.0; // feel free to remove
        let mut cameraX = 0.0;
        let mut cameraY = 0.0;
        let mut cameraZ = 0.0;
        let mut angleX = 0.0;
        let mut angleY = 0.0;

        let camera_speed = 2.0;

        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut previous_frame_time = first_frame_time;

        // upload transformation matrix to the currently active shader
        // unsafe {
        //     let loc = simple_shader.get_uniform_location("transformation_matrix");
        //     gl::UniformMatrix4fv(loc, 1, gl::FALSE, transformation_matrix.as_ptr());
        // }
        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(previous_frame_time).as_secs_f32();
            previous_frame_time = now;


            // Handle resize events
            if let Ok(mut new_size) = window_size.lock() {
                if new_size.2 {
                    context.resize(glutin::dpi::PhysicalSize::new(new_size.0, new_size.1));
                    window_aspect_ratio = new_size.0 as f32 / new_size.1 as f32;
                    (*new_size).2 = false;
                    println!("Window was resized to {}x{}", new_size.0, new_size.1);
                    unsafe {
                        gl::Viewport(0, 0, new_size.0 as i32, new_size.1 as i32);
                    }
                }
            }

            // Handle keyboard input
            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    match key {
                        // The `VirtualKeyCode` enum is defined here:
                        //    https://docs.rs/winit/0.25.0/winit/event/enum.VirtualKeyCode.html
                        VirtualKeyCode::W => {
                            cameraZ += delta_time*camera_speed;
                        }
                        VirtualKeyCode::S => {
                            cameraZ -= delta_time*camera_speed;
                        }
                        VirtualKeyCode::A => {
                            cameraX += delta_time*camera_speed;
                        }
                        VirtualKeyCode::D => {
                            cameraX -= delta_time*camera_speed;
                        }
                        VirtualKeyCode::Space => {
                            cameraY += delta_time*camera_speed;
                        }
                        VirtualKeyCode::LShift => {
                            cameraX -= delta_time*camera_speed;
                        }
                        VirtualKeyCode::Left => {
                            angleY += delta_time*camera_speed;
                        }
                        VirtualKeyCode::Up => {
                            angleX += delta_time*camera_speed;
                        }
                        VirtualKeyCode::Right => {
                            angleY -= delta_time*camera_speed;
                        }
                        VirtualKeyCode::Down => {
                            angleX -= delta_time*camera_speed;
                        }

                        // default handler:
                        _ => {}
                    }
                }
            }
            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {
                // == // Optionally access the accumulated mouse movement between
                // == // frames here with `delta.0` and `delta.1`

                *delta = (0.0, 0.0); // reset when done
            }

            // == // Please compute camera transforms here (exercise 2 & 3)

            let mut camera_transformation_matrix : glm::Mat4 = glm::identity();
            let camera_translation_matrix: glm::Mat4 =
                glm::translation(&glm::vec3(cameraX, cameraY, cameraZ));
            let camera_rotation_matrix_x: glm::Mat4 = 
                glm::rotation(-angleX, &glm::vec3(1.0, 0.0, 0.0));
            
            let camera_rotation_matrix_y: glm::Mat4 = 
                glm::rotation(-angleY, &glm::vec3(0.0, 1.0, 0.0));

            let camera_projection_matrix: glm::Mat4 =
                glm::perspective(
                    window_aspect_ratio,
                    (std::f32::consts::PI)/2.0,
                    1.0,
                    1000.0,
                );

            // The final transformation matrix is a combination of all the previously set transformation matrix
            let camera_transformation_matrix =
                camera_projection_matrix
                * camera_rotation_matrix_y
                * camera_rotation_matrix_x
                * camera_translation_matrix;

            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                // // Assignment 3 Task 1

                // gl::BindVertexArray(vao_body);

                // gl::DrawElements(
                //     gl::TRIANGLES,
                //     helicopter_mesh.body.indices.len() as i32,
                //     gl::UNSIGNED_INT,
                //     ptr::null(),
                // );

                // gl::BindVertexArray(vao_door);

                // gl::DrawElements(
                //     gl::TRIANGLES,
                //     helicopter_mesh.door.indices.len() as i32,
                //     gl::UNSIGNED_INT,
                //     ptr::null(),
                // );

                // gl::BindVertexArray(vao_main_rotor);

                // gl::DrawElements(
                //     gl::TRIANGLES,
                //     helicopter_mesh.main_rotor.indices.len() as i32,
                //     gl::UNSIGNED_INT,
                //     ptr::null(),
                // );

                // gl::BindVertexArray(vao_tail_rotor);

                // gl::DrawElements(
                //     gl::TRIANGLES,
                //     helicopter_mesh.tail_rotor.indices.len() as i32,
                //     gl::UNSIGNED_INT,
                //     ptr::null(),
                // );
                
                // gl::BindVertexArray(vao_terrain);

                // gl::DrawElements(
                //     gl::TRIANGLES,
                //     terrain_mesh.indices.len() as i32,
                //     gl::UNSIGNED_INT,
                //     ptr::null(),
                // );
                
                // draw scene here
                main_rotor_node.rotation = glm::vec3(0.0, elapsed, 0.0);
                draw_scene(&scene_graph, &camera_transformation_matrix, &glm::identity(), &simple_shader);
               
            }

            // Display the new color buffer on the display
            context.swap_buffers().unwrap(); // we use "double buffering" to avoid artifacts
        }
    });

    // == //
    // == // From here on down there are only internals.
    // == //

    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events are initially handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                println!(
                    "New window size received: {}x{}",
                    physical_size.width, physical_size.height
                );
                if let Ok(mut new_size) = arc_window_size.lock() {
                    *new_size = (physical_size.width, physical_size.height, true);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: key_state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        }
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle Escape and Q keys separately
                match keycode {
                    Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Q => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            }
            _ => {}
        }
    });
}

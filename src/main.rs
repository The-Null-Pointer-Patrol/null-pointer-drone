use std::{thread, time::Duration};

use crossbeam_channel::unbounded;
use eframe::EventLoopBuilderHook;
use statistics::{DroneApp, DroneAppMessage};
mod statistics;
use winit::platform::wayland::EventLoopBuilderExtWayland;

fn main() {
    let _handle = thread::spawn(|| {
        let (send, recv) = unbounded::<DroneAppMessage>();
        let event_loop_builder: Option<EventLoopBuilderHook> =
            Some(Box::new(|event_loop_builder| {
                event_loop_builder.with_any_thread(true);
            }));
        let native_options = eframe::NativeOptions {
            event_loop_builder,
            ..Default::default()
        };
        eframe::run_native(
            "App",
            native_options,
            Box::new(|cc| Ok(Box::new(DroneApp::new(cc, 4, recv)))),
        )
        .expect("failed to run app");
    });
    let _handle2 = thread::spawn(|| {
        let (send, recv) = unbounded::<DroneAppMessage>();
        let event_loop_builder: Option<EventLoopBuilderHook> =
            Some(Box::new(|event_loop_builder| {
                event_loop_builder.with_any_thread(true);
            }));
        let native_options = eframe::NativeOptions {
            event_loop_builder,
            ..Default::default()
        };
        eframe::run_native(
            "App",
            native_options,
            Box::new(|cc| Ok(Box::new(DroneApp::new(cc, 4, recv)))),
        )
        .expect("failed to run app");
    });
    loop {}
    //{
    //    let id = 5;
    //    thread::spawn(move || {
    //        let event_loop = EventLoopBuilderExtWayland::with_any_thread(true);
    //        let native_options = eframe::NativeOptions::default();
    //        let _ = eframe::run_detached_native(
    //            "My drone app",
    //            &event_loop,
    //            native_options,
    //            Box::new(|cc| Box::new(DroneApp::new(cc, 4, recv))),
    //        );
    //    });
    //}
    //thread::sleep(Duration::from_secs(100));
}

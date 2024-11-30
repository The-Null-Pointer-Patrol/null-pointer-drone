use crossbeam_channel::Receiver;
use eframe::{egui, Frame};
use egui::Context;
use wg_2024::{controller::NodeEvent, drone::DroneOptions, network::NodeId, packet::Packet};

pub enum DroneAppMessage {
    Event(NodeEvent),
    UpdatedPDR(f64),
}

pub struct DroneApp {
    id: NodeId,
    update_receiver: Receiver<DroneAppMessage>,
}

impl eframe::App for DroneApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(format!("Drone #{}", self.id));
            //ui.horizontal(|ui| {
            //    let name_label = ui.label("Your name: ");
            //    ui.text_edit_singleline(&mut self.name)
            //        .labelled_by(name_label.id);
            //});
            //ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            //if ui.button("Increment").clicked() {
            //    self.age += 1;
            //}
            //ui.label(format!("Hello '{}', age {}", self.name, self.age));
        });
    }
}
impl DroneApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        id: NodeId,
        recv: Receiver<DroneAppMessage>,
    ) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self {
            id,
            update_receiver: recv,
        }
    }
}

//! Shared UI widget helpers.

/// Draws a framed group box with a title label embedded in the top border.
pub fn titled_group<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let title_height = ui.text_style_height(&egui::TextStyle::Body);
    ui.add_space(title_height / 2.0);

    let frame = egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(10, 8));
    let ret = ui.push_id(title, |ui| {
        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui)
        })
    });
    let frame_rect = ret.inner.response.rect;

    let painter = ui.painter();
    let text_color = ui.visuals().weak_text_color();
    let galley = painter.layout_no_wrap(
        title.to_owned(),
        egui::TextStyle::Body.resolve(ui.style()),
        text_color,
    );
    let title_min = egui::pos2(
        frame_rect.left() + 8.0,
        frame_rect.top() - title_height / 2.0,
    );
    let mask_rect = egui::Rect::from_min_size(
        title_min - egui::vec2(2.0, 0.0),
        galley.size() + egui::vec2(4.0, 0.0),
    );
    painter.rect_filled(mask_rect, 0.0, ui.visuals().window_fill());
    painter.galley(title_min, galley, text_color);

    ret.inner.inner
}

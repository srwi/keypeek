//! Small shared egui widget helpers, used by the settings window and the
//! keymap editor alike.

/// A fieldset-style boundary around one group of related controls: a stroked
/// frame spanning the pane width, with `title` embedded in its top border the
/// way an HTML fieldset renders its legend.
pub fn titled_group<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let title_height = ui.text_style_height(&egui::TextStyle::Body);
    // Half the title strip rises above the top border; reserve room for it.
    ui.add_space(title_height / 2.0 + 2.0);

    let frame = egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(10, 8));
    let ret = frame.show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        add_contents(ui)
    });
    let frame_rect = ret.response.rect;

    // Paint the title over the top border, masking the border line behind it
    // with the window background so the line appears to break around the text.
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

    ret.inner
}

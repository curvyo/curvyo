//! The words of the Layers panel and of the layer refusals on the canvas
//! (`specs/0050-layers-and-objects-panel` criteria 23, 32, 36 and 41, the UX notes).

/// "Layer Cut is locked. Unlock it or choose another layer." or the hidden counterpart: drawing
/// into the active layer is refused (criterion 23).
#[must_use]
pub fn draw_refused_notice(layer: &str, locked: bool) -> String {
    if locked {
        format!("Layer {layer} is locked. Unlock it or choose another layer.")
    } else {
        format!("Layer {layer} is hidden. Show it or choose another layer.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_draw_refusals_name_the_layer_and_the_way_out() {
        assert_eq!(
            draw_refused_notice("Cut", true),
            "Layer Cut is locked. Unlock it or choose another layer."
        );
        assert_eq!(
            draw_refused_notice("Cut", false),
            "Layer Cut is hidden. Show it or choose another layer."
        );
    }
}

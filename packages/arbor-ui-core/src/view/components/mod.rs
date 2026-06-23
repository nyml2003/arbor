mod button;
mod image;
mod node;
mod primitive;
mod row;
mod shared;
mod surface;
mod text;

pub use button::{button, ripple, Button, ButtonIntent, ButtonState, RippleVisual};
pub use image::{image, Image};
pub use node::{ComponentBase, ComponentNode, ComponentProps};
pub use primitive::Primitive;
pub use row::{row, Row};
pub use shared::{Align, Border};
pub use surface::{surface, Surface};
pub use text::{text, Text, TextWeight};

#[cfg(test)]
mod tests {
    use crate::geometry::{Point, Rect};
    use crate::theme::ColorToken;

    use super::button::ButtonProps;
    use super::image::ImageProps;
    use super::row::RowProps;
    use super::surface::SurfaceProps;
    use super::text::TextProps;
    use super::{
        button, image, ripple, row, surface, text, Align, ButtonIntent, ButtonState, ComponentNode,
        ComponentProps, Primitive, TextWeight,
    };

    fn assert_props_base(props: &impl ComponentProps, id: &str, rect: Rect) {
        assert_eq!(props.base().id, id);
        assert_eq!(props.base().rect, rect);
    }

    #[test]
    fn button_builder_defaults_to_standard_button_with_empty_text_content() {
        let rect = Rect::new(1.0, 2.0, 3.0, 4.0);
        let primitive = button("button", rect).build();

        let Primitive::Button(button) = primitive else {
            panic!("expected button primitive");
        };
        assert_eq!(button.id(), "button");
        assert_eq!(button.rect(), rect);
        assert_eq!(button.state(), ButtonState::Normal);
        assert_eq!(button.props.intent, ButtonIntent::Standard);
        assert!(button.ripples().is_empty());
        assert!(
            matches!(button.content.as_ref(), Primitive::Text(text) if text.content().is_empty())
        );
    }

    #[test]
    fn builders_create_nested_primitive_tree() {
        let rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let primitive = surface("surface", rect)
            .background(ColorToken::Button)
            .border(ColorToken::Border, 1.0)
            .radius(8.0)
            .children([row("row", rect)
                .gap(4.0)
                .align(Align::Center)
                .children([button("button", rect)
                    .state(ButtonState::Pressed)
                    .intent(ButtonIntent::Action)
                    .ripples([ripple(Point::new(1.0, 2.0), 3.0, 0.4, ColorToken::Ripple)])
                    .child(
                        image("close-icon", rect)
                            .tint(ColorToken::TextPrimary)
                            .build(),
                    )
                    .build()])
                .build()])
            .build();

        let Primitive::Surface(surface) = primitive else {
            panic!("expected surface primitive");
        };
        assert_eq!(surface.id(), "surface");
        assert_eq!(surface.background(), ColorToken::Button);
        assert_eq!(surface.border().expect("surface border").width, 1.0);
        assert_eq!(surface.radius(), 8.0);
        assert_eq!(surface.children().len(), 1);
        let Primitive::Row(row) = &surface.children()[0] else {
            panic!("expected row child");
        };
        assert_eq!(row.props.gap, 4.0);
        assert_eq!(row.props.align, Align::Center);
        assert_eq!(row.children().len(), 1);

        let Primitive::Button(button) = &row.children()[0] else {
            panic!("expected button child");
        };
        assert_eq!(button.state(), ButtonState::Pressed);
        assert_eq!(button.props.intent, ButtonIntent::Action);
        assert_eq!(button.ripples().len(), 1);

        let Primitive::Image(image) = button.content.as_ref() else {
            panic!("expected image content");
        };
        assert_eq!(image.tint(), Some(ColorToken::TextPrimary));
        assert_eq!(image.opacity(), 1.0);
    }

    #[test]
    fn each_atomic_component_exposes_component_node_metadata() {
        let rect = Rect::new(2.0, 4.0, 8.0, 16.0);
        let nodes = [
            surface("surface", rect).build(),
            row("row", rect).build(),
            button("button", rect).build(),
            text("text", rect).content("Label").build(),
            image("image", rect).build(),
        ];

        let expected = ["surface", "row", "button", "text", "image"];

        for (node, id) in nodes.iter().zip(expected) {
            assert_eq!(node.id(), id);
            assert_eq!(node.rect(), rect);
        }
    }

    #[test]
    fn each_atomic_component_has_shared_props_contract() {
        let rect = Rect::new(2.0, 4.0, 8.0, 16.0);

        let button_props = ButtonProps::new("button-props", rect);
        let text_props = TextProps::new("text-props", rect);
        let image_props = ImageProps::new("image-props", rect);
        let row_props = RowProps::new("row-props", rect);
        let surface_props = SurfaceProps::new("surface-props", rect);

        assert_props_base(&button_props, "button-props", rect);
        assert_eq!(button_props.state, ButtonState::Normal);
        assert_eq!(button_props.intent, ButtonIntent::Standard);

        assert_props_base(&text_props, "text-props", rect);
        assert!(text_props.content.is_empty());
        assert_eq!(text_props.style.weight, TextWeight::Regular);
        assert_eq!(text_props.align, Align::Start);

        assert_props_base(&image_props, "image-props", rect);
        assert_eq!(image_props.tint, None);
        assert_eq!(image_props.opacity, 1.0);

        assert_props_base(&row_props, "row-props", rect);
        assert_eq!(row_props.gap, 0.0);
        assert_eq!(row_props.align, Align::Start);

        assert_props_base(&surface_props, "surface-props", rect);
        assert_eq!(surface_props.background, ColorToken::Surface);
        assert_eq!(surface_props.border, None);
        assert_eq!(surface_props.radius, 0.0);
    }

    #[test]
    fn text_builder_keeps_chainable_dsl_with_owned_reconstruction() {
        let rect = Rect::new(0.0, 0.0, 80.0, 24.0);
        let primitive = text("label", rect)
            .content("Apply")
            .color(ColorToken::TextPrimary)
            .size(18.0)
            .weight(TextWeight::Semibold)
            .align(Align::Start)
            .build();

        let Primitive::Text(text) = primitive else {
            panic!("expected text primitive");
        };
        assert_eq!(text.content(), "Apply");
        assert_eq!(text.style().color, ColorToken::TextPrimary);
        assert_eq!(text.style().size, 18.0);
        assert_eq!(text.style().weight, TextWeight::Semibold);
        assert_eq!(text.align(), Align::Start);
    }
}

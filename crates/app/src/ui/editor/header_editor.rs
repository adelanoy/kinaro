use crate::ui::components::dialog::{show_edit_desc_dialog, show_rename_dialog};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::label::Label;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::text::markdown;
use gpui_kit::component::{ActiveTheme, Colorize, Sizable, WindowExt, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{App, ClipboardItem, Entity, IntoElement, ParentElement, RenderOnce, Styled, TextAlign, Window, div};
use ki_assets::icon::IconAsset;
use ki_workspace::test::{TestMetadata, TestsContainer};

/// Header shared by the editors of every kind of test node: shows its name, id
/// and path (both copyable), its active status and its markdown description,
/// with controls to rename it, toggle it and edit its description.
#[derive(IntoElement)]
pub struct HeaderEditor {
  /// Metadata of the edited node, as of when the header was built
  meta: TestMetadata,
  /// Container holding the edited node, through which edits are applied
  tests: Entity<TestsContainer>,
}

impl HeaderEditor {
  /// Creates the header of the node described by `meta`, held in `tests`.
  pub fn new(meta: TestMetadata, tests: Entity<TestsContainer>) -> Self {
    Self { meta, tests }
  }
}

impl RenderOnce for HeaderEditor {
  fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
    let path = self.meta.path();
    let id = self.meta.id();
    let TestMetadata {
      name,
      description,
      disabled,
      ..
    } = self.meta;
    let tests = self.tests.clone();

    v_flex()
      .bg(cx.theme().background.darken(0.3))
      .w_full()
      .p_2()
      .gap_y_2()
      .child(
        h_flex()
          .w_full()
          .justify_between()
          .child(
            //Name and Id
            v_flex()
              .child(
                h_flex().gap_x_2().child(Label::new(name.clone()).text_xl()).child(
                  Button::new("rename-trigger")
                    .icon(IconAsset::Pencil)
                    .ghost()
                    .small()
                    .on_click({
                      let tests = tests.clone();
                      move |_, window, cx| {
                        show_rename_dialog(name.clone(), path, tests.clone(), window, cx);
                      }
                    }),
                ),
              )
              .child(
                Button::new("id-copy")
                  .child(
                    Label::new(format!("id: {id}"))
                      .w_full()
                      .text_align(TextAlign::Left)
                      .text_color(cx.theme().muted_foreground),
                  )
                  .text()
                  .compact()
                  .on_click({
                    move |_, window, cx| {
                      cx.write_to_clipboard(ClipboardItem::new_string(id.to_string()));
                      window.push_notification(Notification::info("Id copied!"), cx);
                    }
                  }),
              )
              .child(
                Button::new("path-copy")
                  .child(
                    Label::new(format!("path: {path}"))
                      .w_full()
                      .text_align(TextAlign::Left)
                      .text_color(cx.theme().muted_foreground),
                  )
                  .text()
                  .compact()
                  .on_click(move |_, window, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(path.to_string()));
                    window.push_notification(Notification::info("Path copied!"), cx);
                  }),
              ),
          )
          // Active switch
          .child(
            div().h_full().child(
              Switch::new("active-switch")
                .label("Active")
                .checked(!disabled)
                .on_click(move |_, _, cx| {
                  self.tests.update(cx, |tests, cx| tests.switch_node_enable_status(&path, cx));
                }),
            ),
          ),
      )
      // Description
      .child(
        div()
          .w_full()
          .child(h_flex().gap_x_2().child(Label::new("Description").text_xl()).child(
            Button::new("desc-trigger").icon(IconAsset::Pencil).ghost().small().on_click({
              let description = description.clone().unwrap_or_default();
              move |_, window, cx| {
                show_edit_desc_dialog(description.clone(), path, tests.clone(), window, cx);
              }
            }),
          ))
          .when_some(description, |this, description| {
            this.child(
              div()
                .pl_1()
                .w_full()
                .h_40()
                .border_color(cx.theme().border)
                .border_1()
                .rounded_md()
                .child(markdown(description.clone()).scrollable(true)),
            )
          }),
      )
  }
}

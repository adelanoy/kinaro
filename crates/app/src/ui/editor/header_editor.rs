use crate::app;
use crate::ui::components::dialog::{show_edit_desc_dialog, show_rename_dialog};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::collapsible::Collapsible;
use gpui_kit::component::label::Label;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::text::markdown;
use gpui_kit::component::{ActiveTheme, IconName, Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{Context, Entity, IntoElement, ParentElement, Render, Styled, TextAlign, Window, div};
use ki_assets::icon::IconAsset;
use ki_workspace::test::TestMetadataOwner;

/// Header shared by the editors of every kind of test node: shows its name, id
/// and path (both copyable), its active status and its markdown description,
/// with controls to rename it, toggle it and edit its description.
pub struct HeaderEditor<M: TestMetadataOwner + 'static> {
  /// The edited node, read when the header is rendered
  meta: Entity<M>,
  content_expanded: bool,
}

impl<M: TestMetadataOwner> HeaderEditor<M> {
  /// Creates the header of the node `meta`.
  pub fn new(meta: Entity<M>) -> Self {
    Self {
      meta,
      content_expanded: false,
    }
  }
}

impl<M: TestMetadataOwner + 'static> Render for HeaderEditor<M> {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let (path, name, description, disabled) = self
      .meta
      .read_with(cx, |meta, _| (meta.path(), meta.name(), meta.description(), meta.disabled()));
    let id = path.id();

    v_flex().w_full().p_2().gap_y_2().child(
      Collapsible::new()
        .open(self.content_expanded)
        .w_full()
        .child(
          h_flex()
            .justify_between()
            .child(
              h_flex().gap_x_2().child(Label::new(name.clone()).text_xl()).child(
                Button::new("rename-trigger")
                  .icon(IconAsset::Pencil)
                  .ghost()
                  .small()
                  .on_click({
                    let meta = self.meta.clone();
                    move |_, window, cx| {
                      show_rename_dialog(name.clone(), meta.clone(), window, cx);
                    }
                  }),
              ),
            )
            .child(
              h_flex()
                .gap_x_2()
                .flex_row_reverse()
                .child(
                  Button::new("expand-desc")
                    .ghost()
                    .when_else(
                      self.content_expanded,
                      |this| this.icon(IconName::ChevronUp),
                      |this| this.icon(IconName::ChevronDown),
                    )
                    .on_click(cx.listener(|this, _, _, _| this.content_expanded = !this.content_expanded)),
                )
                .child(
                  Switch::new("active-switch")
                    .label("Active")
                    .small()
                    .checked(!disabled)
                    .on_click({
                      let meta_owner = self.meta.clone();
                      move |_, _, cx| {
                        meta_owner.update(cx, |meta, cx| meta.switch_disabled(cx));
                      }
                    }),
                ),
            ),
        )
        .content(
          v_flex()
            .child(
              h_flex().w_full().items_start().child(
                Button::new("id-copy")
                  .child(
                    Label::new(format!("id: {id}"))
                      .text_align(TextAlign::Left)
                      .text_color(cx.theme().muted_foreground),
                  )
                  .text()
                  .compact()
                  .on_click(move |_, window, cx| app::copy_id(path, window, cx)),
              ),
            )
            .child(
              h_flex().w_full().items_start().child(
                Button::new("path-copy")
                  .child(
                    Label::new(format!("path: {path}"))
                      .text_align(TextAlign::Left)
                      .text_color(cx.theme().muted_foreground),
                  )
                  .text()
                  .compact()
                  .on_click(move |_, window, cx| app::copy_path(path, window, cx)),
              ),
            )
            .child(h_flex().gap_x_2().child(Label::new("Description").text_xl()).child(
              Button::new("desc-trigger").icon(IconAsset::Pencil).ghost().small().on_click({
                let description = description.clone().unwrap_or_default();
                let meta_owner = self.meta.clone();
                move |_, window, cx| {
                  show_edit_desc_dialog(description.clone(), meta_owner.clone(), window, cx);
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
        ),
    )

    /*v_flex()
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
                    let meta = self.meta.clone();
                    move |_, window, cx| {
                      show_rename_dialog(name.clone(), meta.clone(), window, cx);
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
                .on_click(move |_, window, cx| app::copy_id(path, window, cx)),
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
                .on_click(move |_, window, cx| app::copy_path(path, window, cx)),
            ),
        )
        // Active switch
        .child(
          div().h_full().child(
            Switch::new("active-switch")
              .label("Active")
              .small()
              .checked(!disabled)
              .on_click({
                let meta_owner = self.meta.clone();
                move |_, _, cx| {
                  meta_owner.update(cx, |meta, cx| meta.switch_disabled(cx));
                }
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
            let meta_owner = self.meta.clone();
            move |_, window, cx| {
              show_edit_desc_dialog(description.clone(), meta_owner.clone(), window, cx);
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
    )*/
  }
}

use crate::actions::{Enter, Escape, MoveDown, MoveLeft, MoveRight, MoveUp};
use ki_workspace::test::TestNodeKind;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{ActiveTheme, Icon, IconName, h_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use ki_assets::icon::IconAsset;
use std::ops::Range;
use uuid::Uuid;

pub const TREE_CONTEXT: &str = "Tree";

#[derive(Debug, Clone)]
pub struct ProjectTreeEntry {
  pub path: Vec<Uuid>,
  pub kind: TestNodeKind,
  pub label: SharedString,
  pub disabled: bool,
  pub parent_disabled: bool,
  pub depth: usize,
  pub leaf: bool,
  pub expanded: bool,
}

impl ProjectTreeEntry {
  pub fn icon(&self) -> Option<Icon> {
    match self.kind {
      TestNodeKind::Suite => Some(Icon::new(IconAsset::TestSuite)),
      TestNodeKind::Case => Some(Icon::new(IconAsset::TestCase)),
      TestNodeKind::Step | TestNodeKind::AnonymousStep => Some(Icon::new(IconAsset::TestStep)),
    }
  }

  pub fn id(&self) -> Uuid {
    self.path[self.path.len() - 1]
  }
}

pub trait KiTreeDelegate: Sized + 'static {
  fn row_count(&self, cx: &App) -> usize;

  fn entry(&self, ix: usize, cx: &Context<KiTreeState<Self>>) -> &ProjectTreeEntry;

  fn entry_render(&self, ix: usize, selected: bool, window: &mut Window, cx: &mut Context<KiTreeState<Self>>) -> impl IntoElement;
}

#[derive(Debug)]
pub enum KiTreeEvent {
  EntryDoubleClicked(usize),
  NodeExpanded(usize),
  NodeCollapsed(usize),
}

#[derive(Debug)]
pub struct KiTreeState<D: KiTreeDelegate> {
  focus_handle: FocusHandle,
  scroll_handle: UniformListScrollHandle,
  delegate: D,
  selected_ix: Option<usize>,
}

impl<D: KiTreeDelegate> KiTreeState<D> {
  #[inline]
  pub fn delegate(&self) -> &D {
    &self.delegate
  }

  #[inline]
  pub fn delegate_mut(&mut self) -> &mut D {
    &mut self.delegate
  }

  #[inline]
  pub fn selected_index(&self) -> Option<usize> {
    self.selected_ix
  }

  pub fn clear_selection(&mut self, cx: &mut Context<KiTreeState<D>>) {
    self.selected_ix = None;
    cx.notify();
  }

  fn on_entry_click(&mut self, e: &ClickEvent, ix: usize, cx: &mut Context<KiTreeState<D>>) {
    cx.stop_propagation();
    if e.click_count() == 2 {
      cx.emit(KiTreeEvent::EntryDoubleClicked(ix));
    } else {
      self.selected_ix = Some(ix);
    }
  }

  fn on_entry_right_click(&mut self, ix: usize, cx: &mut Context<KiTreeState<D>>) {
    cx.stop_propagation();
    self.selected_ix = Some(ix);
  }

  fn on_action_left(&mut self, _action: &MoveLeft, _window: &mut Window, cx: &mut Context<KiTreeState<D>>) {
    if let Some(ix) = self.selected_ix {
      cx.emit(KiTreeEvent::NodeCollapsed(ix));
    }
  }

  fn on_action_right(&mut self, _action: &MoveRight, _window: &mut Window, cx: &mut Context<KiTreeState<D>>) {
    if let Some(ix) = self.selected_ix {
      cx.emit(KiTreeEvent::NodeExpanded(ix));
    }
  }

  fn on_action_up(&mut self, _action: &MoveUp, _window: &mut Window, cx: &mut Context<KiTreeState<D>>) {
    if let Some(ix) = self.selected_ix
      && ix > 0
    {
      self.selected_ix = Some(ix - 1);
      cx.notify();
    }
  }

  fn on_action_down(&mut self, _action: &MoveDown, _window: &mut Window, cx: &mut Context<KiTreeState<D>>) {
    if let Some(ix) = self.selected_ix
      && ix < self.delegate.row_count(cx) - 1
    {
      self.selected_ix = Some(ix + 1);
      cx.notify();
    }
  }

  fn on_action_confirm(&mut self, _action: &Enter, _window: &mut Window, cx: &mut Context<KiTreeState<D>>) {
    if let Some(ix) = self.selected_ix
      && ix < self.delegate.row_count(cx) - 1
    {
      cx.emit(KiTreeEvent::EntryDoubleClicked(ix));
    }
  }
}

impl<D: KiTreeDelegate> Focusable for KiTreeState<D> {
  fn focus_handle(&self, _cx: &App) -> FocusHandle {
    self.focus_handle.clone()
  }
}

impl<D: KiTreeDelegate> KiTreeState<D> {
  pub fn new(delegate: D, cx: &mut Context<Self>) -> Self {
    Self {
      focus_handle: cx.focus_handle(),
      scroll_handle: UniformListScrollHandle::default(),
      delegate,
      selected_ix: None,
    }
  }
}

impl<D: KiTreeDelegate> EventEmitter<KiTreeEvent> for KiTreeState<D> {}

impl<D: KiTreeDelegate> Render for KiTreeState<D> {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div().size_full().child(
      uniform_list(
        "tree_entries",
        self.delegate().row_count(cx),
        cx.processor(move |state, visible_range: Range<usize>, window, cx| {
          visible_range
            .map(|ix| {
              let entry = state.delegate().entry(ix, cx);
              let is_selected = state.selected_ix == Some(ix);
              let is_open = entry.expanded;
              let mut left_padding = px(16.) * entry.depth;
              if entry.leaf {
                // medium icon + x_gap: 14px + 4px
                left_padding += px(18.);
              }
              ListItem::new(ix)
                .w_full()
                .rounded(cx.theme().radius)
                .pl(left_padding)
                .selected(is_selected)
                .child(
                  h_flex()
                    .gap_x_1()
                    .size_full()
                    // Chevron icon
                    .when(!entry.leaf, {
                      let entity = cx.entity();
                      move |this| {
                        let mut icon = Icon::new(IconName::ChevronDown);
                        if !is_open {
                          icon = icon.rotate(percentage(0.75));
                        }
                        this.child(
                          Button::new(ix)
                            .text()
                            .icon(icon)
                            .cursor(CursorStyle::Arrow)
                            .tab_stop(false)
                            .on_click(move |_, _, cx| {
                              entity.update(cx, |_, cx| {
                                use crate::ui::components::tree::KiTreeEvent::NodeCollapsed;
                                use crate::ui::components::tree::KiTreeEvent::NodeExpanded;
                                cx.stop_propagation();
                                if is_open {
                                  cx.emit(NodeCollapsed(ix))
                                } else {
                                  cx.emit(NodeExpanded(ix))
                                }
                              })
                            }),
                        )
                      }
                    })
                    // Entry content
                    .child(state.delegate().entry_render(ix, is_selected, window, cx)),
                )
                .on_click(cx.listener(move |this, e, _, cx| this.on_entry_click(e, ix, cx)))
                .on_mouse_down(
                  MouseButton::Right,
                  cx.listener(move |this, _, _, cx| this.on_entry_right_click(ix, cx)),
                )
            })
            .collect()
        }),
      )
        .flex_grow_1()
        .size_full()
        .track_scroll(&self.scroll_handle),
    )
  }
}

#[derive(IntoElement)]
pub struct KiTree<D: KiTreeDelegate> {
  state: Entity<KiTreeState<D>>,
}

impl<D: KiTreeDelegate> KiTree<D> {
  pub fn new(state: Entity<KiTreeState<D>>) -> Self {
    Self { state }
  }
}

impl<D: KiTreeDelegate> RenderOnce for KiTree<D> {
  fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let scroll_handle = self.state.read(cx).scroll_handle.clone();
    let focus_handle = self.state.read(cx).focus_handle.clone();

    div()
      .id(TREE_CONTEXT)
      .size_full()
      .track_focus(&focus_handle)
      .key_context(TREE_CONTEXT)
      .on_action(window.listener_for(&self.state, |state, _: &Escape, _, cx| state.clear_selection(cx)))
      .on_action(window.listener_for(&self.state, KiTreeState::on_action_left))
      .on_action(window.listener_for(&self.state, KiTreeState::on_action_right))
      .on_action(window.listener_for(&self.state, KiTreeState::on_action_up))
      .on_action(window.listener_for(&self.state, KiTreeState::on_action_down))
      .on_action(window.listener_for(&self.state, KiTreeState::on_action_confirm))
      .child(self.state.clone())
      .vertical_scrollbar(&scroll_handle)
      .overflow_hidden()
  }
}

use crate::actions::{Enter, Escape, MoveDown, MoveLeft, MoveRight, MoveUp};
use ki_workspace::test::TestInfoId;
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
pub struct ProjectTreeNode {
  pub info_id: TestInfoId,
  pub label: SharedString,
  pub disabled: bool,
  pub parent_disabled: bool,
  pub depth: usize,
  pub leaf: bool,
  pub expanded: bool,
}

impl ProjectTreeNode {
  pub fn icon(&self) -> Option<Icon> {
    match self.info_id {
      TestInfoId::Suite(_) => Some(Icon::new(IconAsset::TestSuite)),
      TestInfoId::CaseMulti(_, _) => Some(Icon::new(IconAsset::TestCase)),
      TestInfoId::Step(_, _, _) | TestInfoId::CaseStep(_, _) => Some(Icon::new(IconAsset::TestStep)),
    }
  }

  pub fn id(&self) -> Uuid {
    self.info_id.id()
  }
}

pub trait TreeDelegate: Sized + 'static {
  fn row_count(&self, cx: &App) -> usize;

  fn node(&self, ix: usize, cx: &Context<TreeState<Self>>) -> &ProjectTreeNode;

  fn node_render(&self, ix: usize, selected: bool, window: &mut Window, cx: &mut Context<TreeState<Self>>) -> impl IntoElement;
}

#[derive(Debug)]
pub enum TreeEvent {
  EntryDoubleClicked(usize),
  NodeExpanded(usize),
  NodeCollapsed(usize),
}

#[derive(Debug)]
pub struct TreeState<D: TreeDelegate> {
  focus_handle: FocusHandle,
  scroll_handle: UniformListScrollHandle,
  delegate: D,
  selected_ix: Option<usize>,
}

impl<D: TreeDelegate> TreeState<D> {
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

  pub fn clear_selection(&mut self, cx: &mut Context<TreeState<D>>) {
    self.selected_ix = None;
    cx.notify();
  }

  fn on_node_click(&mut self, e: &ClickEvent, ix: usize, cx: &mut Context<TreeState<D>>) {
    cx.stop_propagation();
    if e.click_count() == 2 {
      cx.emit(TreeEvent::EntryDoubleClicked(ix));
    } else {
      self.selected_ix = Some(ix);
    }
  }

  fn on_node_right_click(&mut self, ix: usize, cx: &mut Context<TreeState<D>>) {
    cx.stop_propagation();
    self.selected_ix = Some(ix);
  }

  fn on_action_left(&mut self, _action: &MoveLeft, _window: &mut Window, cx: &mut Context<TreeState<D>>) {
    if let Some(ix) = self.selected_ix {
      cx.emit(TreeEvent::NodeCollapsed(ix));
    }
  }

  fn on_action_right(&mut self, _action: &MoveRight, _window: &mut Window, cx: &mut Context<TreeState<D>>) {
    if let Some(ix) = self.selected_ix {
      cx.emit(TreeEvent::NodeExpanded(ix));
    }
  }

  fn on_action_up(&mut self, _action: &MoveUp, _window: &mut Window, cx: &mut Context<TreeState<D>>) {
    if let Some(ix) = self.selected_ix
      && ix > 0
    {
      self.selected_ix = Some(ix - 1);
      cx.notify();
    }
  }

  fn on_action_down(&mut self, _action: &MoveDown, _window: &mut Window, cx: &mut Context<TreeState<D>>) {
    if let Some(ix) = self.selected_ix
      && ix < self.delegate.row_count(cx) - 1
    {
      self.selected_ix = Some(ix + 1);
      cx.notify();
    }
  }

  fn on_action_confirm(&mut self, _action: &Enter, _window: &mut Window, cx: &mut Context<TreeState<D>>) {
    if let Some(ix) = self.selected_ix
      && ix < self.delegate.row_count(cx) - 1
    {
      cx.emit(TreeEvent::EntryDoubleClicked(ix));
    }
  }
}

impl<D: TreeDelegate> Focusable for TreeState<D> {
  fn focus_handle(&self, _cx: &App) -> FocusHandle {
    self.focus_handle.clone()
  }
}

impl<D: TreeDelegate> TreeState<D> {
  pub fn new(delegate: D, cx: &mut Context<Self>) -> Self {
    Self {
      focus_handle: cx.focus_handle(),
      scroll_handle: UniformListScrollHandle::default(),
      delegate,
      selected_ix: None,
    }
  }
}

impl<D: TreeDelegate> EventEmitter<TreeEvent> for TreeState<D> {}

impl<D: TreeDelegate> Render for TreeState<D> {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div().size_full().child(
      uniform_list(
        "tree_entries",
        self.delegate().row_count(cx),
        cx.processor(move |state, visible_range: Range<usize>, window, cx| {
          visible_range
            .map(|ix| {
              let node = state.delegate().node(ix, cx);
              let is_selected = state.selected_ix == Some(ix);
              let is_open = node.expanded;
              let mut left_padding = px(16.) * node.depth;
              if node.leaf {
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
                    .when(!node.leaf, {
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
                                use crate::ui::components::tree::TreeEvent::NodeCollapsed;
                                use crate::ui::components::tree::TreeEvent::NodeExpanded;
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
                    .child(state.delegate().node_render(ix, is_selected, window, cx)),
                )
                .on_click(cx.listener(move |this, e, _, cx| this.on_node_click(e, ix, cx)))
                .on_mouse_down(
                  MouseButton::Right,
                  cx.listener(move |this, _, _, cx| this.on_node_right_click(ix, cx)),
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
pub struct Tree<D: TreeDelegate> {
  state: Entity<TreeState<D>>,
}

impl<D: TreeDelegate> Tree<D> {
  pub fn new(state: Entity<TreeState<D>>) -> Self {
    Self { state }
  }
}

impl<D: TreeDelegate> RenderOnce for Tree<D> {
  fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let scroll_handle = self.state.read(cx).scroll_handle.clone();
    let focus_handle = self.state.read(cx).focus_handle.clone();

    div()
      .id(TREE_CONTEXT)
      .size_full()
      .track_focus(&focus_handle)
      .key_context(TREE_CONTEXT)
      .on_action(window.listener_for(&self.state, |state, _: &Escape, _, cx| state.clear_selection(cx)))
      .on_action(window.listener_for(&self.state, TreeState::on_action_left))
      .on_action(window.listener_for(&self.state, TreeState::on_action_right))
      .on_action(window.listener_for(&self.state, TreeState::on_action_up))
      .on_action(window.listener_for(&self.state, TreeState::on_action_down))
      .on_action(window.listener_for(&self.state, TreeState::on_action_confirm))
      .child(self.state.clone())
      .vertical_scrollbar(&scroll_handle)
      .overflow_hidden()
  }
}

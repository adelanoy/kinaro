use gpui_kit::component::Icon;
use gpui_kit::{AnyView, App, Entity, ParentElement, Render, SharedString, Styled, Window};
use gpui_kit::component::tab::Tab;

pub struct PanelContainer<D> {
  selected_ix: usize,
  panels: Vec<Panel<D>>,
}

impl<D: Clone + PartialEq> PanelContainer<D> {
  pub fn new(panels: Vec<Panel<D>>, initial_panel: D) -> Self {
    let selected_ix = panels.iter().position(|panel| panel.panel_type == initial_panel).unwrap_or_default();
    Self {
      selected_ix,
      panels,
    }
  }
  
  #[inline]
  pub fn selected_ix(&self) -> usize {
    self.selected_ix
  }
  
  #[inline]
  pub fn selected_panel(&self) -> D {
    self.panels[self.selected_ix].panel_type.clone()
  }

  #[inline]
  pub fn set_selected_ix(&mut self, ix: usize)  {
    self.selected_ix = ix;
  }
  
  pub fn tabs(&self) -> impl Iterator<Item = Tab> {
    self.panels.iter().map(|panel| {
      Tab::new()
        .px_2()
        .prefix(panel.icon.clone())
        .child(panel.title.clone())
    })
  }
  
  pub fn view(&self) -> AnyView {
    self.panels[self.selected_ix].view.clone()
  }
}

#[derive(Clone)]
pub struct Panel<D> {
  title: SharedString,
  icon: Icon,
  panel_type: D,
  view: AnyView,
}

impl<D: Clone + 'static> Panel<D> {
  pub fn build<E, T: PanelDescriptor<D, E> + Render>(entity: Entity<E>, window: &mut Window, cx: &mut App) -> Self {
    let title = T::name();
    let icon = T::icon();
    let panel_type = T::panel_type();
    let view = T::new(entity, window, cx);

    Self {
      title: SharedString::from(title),
      icon: icon.into(),
      panel_type,
      view: view.into(),
    }
  }
}

pub trait PanelDescriptor<D, E>: Render {
  fn name() -> &'static str;

  fn icon() -> impl Into<Icon>;

  fn panel_type() -> D;

  fn new(entity: Entity<E>, window: &mut Window, cx: &mut App) -> Entity<Self>;
}

use crate::step_editor::rest_editor::RestEditorPanel;
use gpui_kit::base::input::InputEvent;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::separator::Separator;
use gpui_kit::component::{Icon, IndexPath, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, rems};
use ki_app_shared::SubbedEntity;
use ki_app_shared::panel::PanelDescriptor;
use ki_assets::icon::IconAsset;
use ki_utils::http::ContentType;
use ki_workspace::test::step_rest::{RestBody, RestData};

enum EditorMode {
  None,
  Table,
  Text(SubbedEntity<EditorState>),
}

pub struct RestBodyEditor {
  rest_data: Entity<RestData>,
  body_type_select: SubbedEntity<SelectState<Vec<&'static str>>>,
  content_type_select: Option<SubbedEntity<SelectState<Vec<ContentType>>>>,
  editor_mode: EditorMode,
}

impl RestBodyEditor {
  fn set_editor_mode(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let body = self.rest_data.read(cx).body().clone();
    let editor = match body {
      RestBody::None => EditorMode::None,
      RestBody::Raw { content_type, content: lines } => {
        let language = match content_type {
          ContentType::Json => "json",
          ContentType::Xml => "xml",
          ContentType::Csv => "csv",
          ContentType::Text => "text",
          ContentType::Yaml => "yaml",
        };
        if let EditorMode::Text(state) = &self.editor_mode
          && state.entity.read(cx).language_name() == language
        {
          return;
        }
        let editor_state = cx.new(|cx| {
          let mut editor = EditorState::new(window, cx)
            .language(language)
            .auto_close(true)
            .smart_indent(true);
          editor.set_value(lines.clone(), window, cx);
          editor
        });
        let _sub = cx.subscribe(&editor_state, |this, state, e: &InputEvent, cx| {
          if let InputEvent::Change = e {
            this
              .rest_data
              .update(cx, |rest, cx| rest.set_body_raw_content(state.read(cx).value(), cx))
          }
        });
        EditorMode::Text(SubbedEntity {
          entity: editor_state,
          _sub,
        })
      }
      RestBody::FormUrlEncoded { .. } => EditorMode::Table,
      RestBody::Multipart { .. } => EditorMode::Table,
    };
    self.editor_mode = editor;
  }

  fn set_content_type_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let content_type_select = if let RestBody::Raw { content_type, .. } = self.rest_data.read(cx).body() {
      let content_types = ContentType::ALL.to_vec();
      let ix = content_types.iter().position(|ct| ct == content_type).unwrap_or(0);
      let content_type_select = cx.new(|cx| SelectState::new(content_types, Some(IndexPath::new(ix)), window, cx));
      let _sub = cx.subscribe_in(
        &content_type_select,
        window,
        |this, _, e: &SelectEvent<Vec<ContentType>>, window, cx| {
          if let SelectEvent::Confirm(Some(content_type)) = e {
            this
              .rest_data
              .update(cx, |rest, cx| rest.set_body_raw_content_type(*content_type, cx));
            this.set_editor_mode(window, cx);
          }
        },
      );
      Some(SubbedEntity {
        entity: content_type_select,
        _sub,
      })
    } else {
      None
    };
    self.content_type_select = content_type_select;
  }
}

impl PanelDescriptor<RestEditorPanel, RestData> for RestBodyEditor {
  fn name() -> &'static str {
    "Body"
  }

  fn icon() -> impl Into<Icon> {
    IconAsset::Braces
  }

  fn panel_type() -> RestEditorPanel {
    RestEditorPanel::Body
  }

  fn new(rest_data: Entity<RestData>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    let body_type_select = {
      let entity = cx.new(|cx| {
        let body_types = RestBody::ALL.to_vec();
        let current_body = rest_data.read(cx).body().type_as_str();
        let pos = body_types.iter().position(|&bt| bt == current_body);
        SelectState::new(body_types, pos.map(IndexPath::new), window, cx)
      });
      let _sub = cx.subscribe_in(&entity, window, |this, _, e: &SelectEvent<Vec<&'static str>>, window, cx| {
        if let SelectEvent::Confirm(Some(value)) = e {
          let body = RestBody::from(*value);
          this.rest_data.update(cx, |rest_data, cx| rest_data.set_body(body, cx));
          this.set_content_type_select(window, cx);
          this.set_editor_mode(window, cx);
        }
      });
      SubbedEntity { entity, _sub }
    };

    let mut this = RestBodyEditor {
      rest_data,
      body_type_select,
      content_type_select: None,
      editor_mode: EditorMode::None,
    };

    this.set_content_type_select(window, cx);
    this.set_editor_mode(window, cx);
    this
  }
}

impl Render for RestBodyEditor {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    v_flex()
      .p_1()
      .size_full()
      .gap_y_2()
      .child(
        h_flex()
          .w_full()
          .gap_x_2()
          .child(
            div().flex_none().child(
              Select::new(&self.body_type_select.entity)
                .small()
                .menu_width(rems(10.))
                .title_prefix("Body Type: "),
            ),
          )
          .when_some(self.content_type_select.as_ref(), |this, select| {
            this.child(Separator::vertical()).child(
              div()
                .flex_none()
                .child(Select::new(&select.entity).small().title_prefix("Content Type: ")),
            )
          }),
      )
      .child(v_flex().w_full().flex_1().min_h_0().map(|this| match &self.editor_mode {
        EditorMode::None => this,
        EditorMode::Table => this.child("To be implemented"),
        EditorMode::Text(editor) => this.child(Editor::new(&editor.entity).h_full()),
      }))
  }
}

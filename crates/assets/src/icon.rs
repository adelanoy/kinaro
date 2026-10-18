use gpui_kit::component::{Icon, IconNamed};
use gpui_kit::{App, IntoElement, RenderOnce, SharedString, Window};

#[derive(IntoElement, Clone)]
pub enum IconAsset {
  Ban,
  Duplicate,
  Pencil,
  Profile,
  Refresh,
  Rename,
  StepDelay,
  StepRestDelete,
  StepRestGet,
  StepRestHead,
  StepRestOptions,
  StepRestPatch,
  StepRestPost,
  StepRestPut,
  TestCase,
  TestStep,
  TestSuite,
  Tree,
  Variable,
}

impl IconNamed for IconAsset {
  fn path(self) -> SharedString {
    match self {
      IconAsset::Ban => "icons/ban.svg",
      IconAsset::Duplicate => "icons/duplicate.svg",
      IconAsset::Pencil => "icons/pencil.svg",
      IconAsset::Profile => "icons/profile.svg",
      IconAsset::Refresh => "icons/refresh.svg",
      IconAsset::Rename => "icons/rename.svg",
      IconAsset::StepDelay => "icons/step-delay.svg",
      IconAsset::StepRestDelete => "icons/step-rest-delete.svg",
      IconAsset::StepRestGet => "icons/step-rest-get.svg",
      IconAsset::StepRestHead => "icons/step-rest-head.svg",
      IconAsset::StepRestOptions => "icons/step-rest-options.svg",
      IconAsset::StepRestPatch => "icons/step-rest-patch.svg",
      IconAsset::StepRestPost => "icons/step-rest-post.svg",
      IconAsset::StepRestPut => "icons/step-rest-put.svg",
      IconAsset::TestCase => "icons/test-case.svg",
      IconAsset::TestStep => "icons/test-step.svg",
      IconAsset::TestSuite => "icons/test-suite.svg",
      IconAsset::Tree => "icons/tree.svg",
      IconAsset::Variable => "icons/variable.svg",
    }
    .into()
  }
}

impl RenderOnce for IconAsset {
  fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
    Icon::new(self)
  }
}

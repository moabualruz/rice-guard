// Init command engine — language detection, interactive wizard, and config generator.

pub mod detector;
pub mod generator;
pub mod wizard;

pub use detector::{detect, DetectionResult, SccLanguage};
pub use generator::{generate, GeneratorInput};
pub use wizard::{noninteractive_choices, wizard, CiProvider, WizardChoices};

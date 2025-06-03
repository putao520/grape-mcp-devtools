pub mod detector;
pub mod registry;
pub mod tool_installer;

pub use detector::{CliDetector, CliToolInfo};
pub use registry::{DynamicToolRegistry, RegistrationStrategy, RegistrationReport};
pub use tool_installer::{
    ToolInstaller, ToolInstallConfig, InstallStrategy, ToolInstallInfo, 
    InstallMethod, InstallationReport, UpgradeReport
};

 
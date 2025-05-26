use serde::{Deserialize, Serialize};
use std::fmt;

/// 包管理器类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Registry {
    /// Rust包管理器(crates.io)
    Cargo,
    /// Python包管理器(PyPI)
    PyPI,
    /// JavaScript/TypeScript包管理器(npm)
    Npm,
    /// Java包管理器(Maven Central)
    Maven,
    /// Java包管理器(Gradle)
    Gradle,
    /// Go包管理器(proxy.golang.org)
    Go,
    /// Dart/Flutter包管理器(pub.dev)
    Pub,
    /// .NET包管理器(NuGet)
    NuGet,
}

impl Registry {
    /// 获取注册表的基础URL
    pub fn base_url(&self) -> &str {
        match self {
            Registry::Cargo => "https://crates.io/api/v1",
            Registry::PyPI => "https://pypi.org/pypi",
            Registry::Npm => "https://registry.npmjs.org",
            Registry::Maven => "https://search.maven.org/solrsearch/select",
            Registry::Gradle => "https://plugins.gradle.org/api",
            Registry::Go => "https://proxy.golang.org",
            Registry::Pub => "https://pub.dev/api",
            Registry::NuGet => "https://api.nuget.org/v3",
        }
    }

    /// 获取包的主页URL
    pub fn package_url(&self, name: &str) -> String {
        match self {
            Registry::Cargo => format!("https://crates.io/crates/{}", name),
            Registry::PyPI => format!("https://pypi.org/project/{}", name),
            Registry::Npm => format!("https://www.npmjs.com/package/{}", name),
            Registry::Maven => format!("https://search.maven.org/artifact/{}", name),
            Registry::Gradle => format!("https://plugins.gradle.org/plugin/{}", name),
            Registry::Go => format!("https://pkg.go.dev/{}", name),
            Registry::Pub => format!("https://pub.dev/packages/{}", name),
            Registry::NuGet => format!("https://www.nuget.org/packages/{}", name),
        }
    }
}

impl fmt::Display for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Registry::Cargo => write!(f, "cargo"),
            Registry::PyPI => write!(f, "pip"),
            Registry::Npm => write!(f, "npm"),
            Registry::Maven => write!(f, "maven"),
            Registry::Gradle => write!(f, "gradle"),
            Registry::Go => write!(f, "go"),
            Registry::Pub => write!(f, "pub"),
            Registry::NuGet => write!(f, "nuget"),
        }
    }
}

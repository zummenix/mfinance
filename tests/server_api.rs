//! Container-based tests - requires Podman and `cargo test --features container-tests`
#![cfg(feature = "container-tests")]

use rustainers::{
    ImageName, Volume, WaitStrategy,
    images::GenericImage,
    runner::{RunOption, Runner},
};

use temp_dir::TempDir;

#[tokio::test]
async fn test_api_root() -> Result<(), Box<dyn std::error::Error>> {
    let host_dir = HostDir::new()?;
    let container = RunningContainer::new(&host_dir).await?;

    let resp = reqwest::get(&container.endpoint("/")).await?;
    assert_eq!(resp.status(), 200);

    let body = resp.text().await?;
    assert!(body.contains("mfinance"));

    Ok(())
}

#[tokio::test]
async fn test_api_files() -> Result<(), Box<dyn std::error::Error>> {
    let host_dir = HostDir::new()?;
    host_dir.write_file("test1.csv", "")?;
    host_dir.write_file("test3.txt", "")?;
    host_dir.write_file("test2.csv", "")?;

    let container = RunningContainer::new(&host_dir).await?;

    let resp = reqwest::get(&container.endpoint("/api/files")).await?;

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await?;
    insta::assert_json_snapshot!(body, @r#"
    [
      "test1.csv",
      "test2.csv"
    ]
    "#);

    Ok(())
}

#[tokio::test]
async fn test_api_file() -> Result<(), Box<dyn std::error::Error>> {
    let host_dir = HostDir::new()?;
    host_dir.write_file(
        "test.csv",
        "date;amount\n\
         2024-01-01;100\n\
         2024-02-15;-50.5\n\
         2025-03-01;200.25\n\
         2024-03-10;75",
    )?;

    let container = RunningContainer::new(&host_dir).await?;
    let resp = reqwest::get(&container.endpoint("/api/files/test.csv")).await?;

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await?;
    insta::assert_json_snapshot!(body, @r#"
    {
      "total": "324.75",
      "years": [
        {
          "entries": [
            {
              "amount": "100.00",
              "date": "January 1"
            },
            {
              "amount": "-50.50",
              "date": "February 15"
            },
            {
              "amount": "75.00",
              "date": "March 10"
            }
          ],
          "subtotal": "124.50",
          "year": "2024"
        },
        {
          "entries": [
            {
              "amount": "200.25",
              "date": "March 1"
            }
          ],
          "subtotal": "200.25",
          "year": "2025"
        }
      ]
    }
    "#);

    Ok(())
}

#[tokio::test]
async fn test_api_file_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let host_dir = HostDir::new()?;
    let container = RunningContainer::new(&host_dir).await?;

    let resp = reqwest::get(&container.endpoint("/api/files/nonexistent.csv")).await?;
    assert_eq!(resp.status(), 500);

    Ok(())
}

struct HostDir {
    temp_dir: TempDir,
}

impl HostDir {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(HostDir {
            temp_dir: TempDir::with_prefix("mfinance-server-")?,
        })
    }

    fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    fn write_file(&self, name: &str, content: &str) -> Result<(), std::io::Error> {
        std::fs::write(self.path().join(name), content)
    }
}

struct RunningContainer {
    _runner: Runner,
    _container: rustainers::Container<GenericImage>,
    server_url: String,
}

impl RunningContainer {
    async fn new(host_dir: &HostDir) -> Result<RunningContainer, Box<dyn std::error::Error>> {
        let port: u16 = 3000;
        let runner = Runner::podman()?;
        let mut image = GenericImage::new("localhost/mfinance".parse::<ImageName>()?);
        image.set_wait_strategy(WaitStrategy::scan_port(port));
        image.add_port_mapping(port);

        let container = runner
            .start_with_options(
                image,
                RunOption::builder()
                    .with_volumes([Volume::bind_mount(
                        host_dir.path().to_string_lossy().into_owned(),
                        "/data",
                    )])
                    .with_remove(true)
                    .build(),
            )
            .await?;

        let host_port = container.host_port(port).await?;

        Ok(RunningContainer {
            _runner: runner,
            _container: container,
            server_url: format!("http://localhost:{host_port}"),
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{path}", self.server_url)
    }
}

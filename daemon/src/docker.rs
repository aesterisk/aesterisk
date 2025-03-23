use bollard::Docker;
use tokio::sync::OnceCell;

pub mod network;
pub mod server;

static DOCKER: OnceCell<Docker> = OnceCell::const_new();

pub fn init() -> Result<(), String> {
    let docker = Docker::connect_with_local_defaults().map_err(|e| format!("Could not connect to socket: {}", e))?;
    DOCKER.set(docker).map_err(|_| "Docker has already been initialised")?;
    Ok(())
}

pub fn get() -> Result<&'static Docker, String> {
    Ok(DOCKER.get().ok_or("Docker has not been initialised")?)
}

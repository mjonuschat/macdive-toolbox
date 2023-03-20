use crate::helpers::fs::create_dir;
use crate::types::APPLICATION_NAME;
use anyhow::{anyhow, Result};
use sea_orm::{Database, DbConn};
use tokio::sync::OnceCell;

static DB_CONN: OnceCell<DbConn> = OnceCell::const_new();

pub(crate) fn create_db() -> Result<String> {
    let path = dirs::data_dir()
        .ok_or(anyhow!("Error determining data dir for application"))?
        .join(APPLICATION_NAME);

    create_dir(&path)?;
    let db_path = path.join("toolbox.sqlite");
    if std::fs::metadata(&db_path).is_err() {
        std::fs::File::create(&db_path)?;
    }
    Ok(format!("sqlite://{}", &db_path.to_string_lossy()))
}

pub async fn connect() -> Result<&'static DbConn> {
    Ok(DB_CONN
        .get_or_init(|| async {
            let url = create_db().unwrap();
            Database::connect(url).await.unwrap()
        })
        .await)
}

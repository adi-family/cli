use anyhow::Context;

fn main() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        dotenvy::dotenv().ok();

        let database_url = lib_env_parse::env_opt("DATABASE_URL")
            .context("DATABASE_URL is required")?;

        let pool = sqlx::PgPool::connect(&database_url).await?;
        sqlx::migrate!("../core/migrations").run(&pool).await?;

        println!("Migrations applied successfully");
        Ok(())
    })
}

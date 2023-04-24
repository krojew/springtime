use springtime::application;
use springtime_migrate_refinery::migration::embed_migrations;

// this is all that's needed to embed sql migrations from the given folder (the default path is
// "migrations")
// when building this example, the current working directory is the workspace one
embed_migrations!("./springtime-migrate-refinery/examples/migrations");

// note: for the sake of simplicity, errors are unwrapped, rather than gracefully handled
#[tokio::main]
async fn main() {
    // create our application, which will run refinery migration runner before other runners
    let mut application =
        application::create_default().expect("unable to create default application");

    // will run migrations from the "migrations" folder if a supported db client provider component
    // is available (not shown in this example)
    application.run().await.expect("error running application");
}

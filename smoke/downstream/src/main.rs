// Touches both halves of the contract: the Slint components come in through
// `@nodeeditor`, and `LinkData` — which lives in the library crate now, not in
// this crate's generated code — comes in through the Rust re-export.
use slint::{ModelRc, VecModel};
use slint_node_editor::LinkData;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = App::new()?;
    app.set_links(ModelRc::new(VecModel::from(vec![LinkData {
        id: 1,
        start_pin_id: 2,
        end_pin_id: 3,
        line_width: 2.0,
        ..Default::default()
    }])));

    // Headless: proving it compiles and instantiates is the whole job.
    if std::env::var_os("SMOKE_RUN").is_some() {
        app.run()?;
    }
    Ok(())
}

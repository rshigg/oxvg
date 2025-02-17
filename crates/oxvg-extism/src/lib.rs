use extism_pdk::*;
use oxvg_ast::implementations::markup5ever::{Element5Ever, Node5Ever};
use oxvg_ast::parse::Node;
use oxvg_optimiser::Jobs;

#[plugin_fn]
pub fn optimise(string: String) -> FnResult<String> {
    use oxvg_ast::serialize::Node;

    // Parse the SVG string into AST
    let dom = Node5Ever::parse(&string)?;

    // Create default optimization jobs
    let jobs: Jobs<Element5Ever> = Jobs::default();

    // Run the optimization jobs
    jobs.run(
        &dom,
        &oxvg_ast::visitor::Info {
            path: None,
            multipass_count: 0,
        },
    )?;

    // Serialize the optimized DOM back to string
    let result = dom.serialize()?;
    Ok(result)
}

// // use json data for inputs and outputs
// #[derive(FromBytes, Deserialize, PartialEq, Debug)]
// #[encoding(Json)]
// struct Add {
//     left: i32,
//     right: i32,
// }
// #[derive(ToBytes, Serialize, PartialEq, Debug)]
// #[encoding(Json)]
// struct Sum {
//     value: i32,
// }

// #[plugin_fn]
// pub fn add(input: Add) -> FnResult<Sum> {
//     Ok(Sum {
//         value: input.left + input.right,
//     })
// }

// A large cogwheel's teeth reach past its block.
fn create_bounds(block: &str, bounds: &mut [f32; 6]) {
    if block == "create:large_cogwheel" {
        *bounds = [-1.0, -1.0, -1.0, 2.0, 2.0, 2.0];
    }
}

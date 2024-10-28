use ethers_flashbots::BundleRequest;

pub fn clone_bundle_request_without_txs(readonly_bundle: &BundleRequest) -> BundleRequest {
    let mut bundle = BundleRequest::new();
    if let Some(x) = readonly_bundle.block() {
        bundle = bundle.set_block(x);
    }
    if let Some(x) = readonly_bundle.simulation_block() {
        bundle = bundle.set_simulation_block(x);
    }
    if let Some(x) = readonly_bundle.simulation_timestamp() {
        bundle = bundle.set_simulation_timestamp(x);
    }
    if let Some(x) = readonly_bundle.min_timestamp() {
        bundle = bundle.set_min_timestamp(x);
    }
    if let Some(x) = readonly_bundle.max_timestamp() {
        bundle = bundle.set_max_timestamp(x);
    }
    bundle
}

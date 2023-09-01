use soroban_ledger_snapshot::LedgerSnapshot;

use crate::soroban_cli::network::sandbox_network_id;

pub fn empty_ledger_snapshot() -> LedgerSnapshot {
    LedgerSnapshot {
        network_id: sandbox_network_id(),
        min_persistent_entry_expiration: u32::MAX,
        min_temp_entry_expiration: u32::MAX,
        ..Default::default()
    }
}

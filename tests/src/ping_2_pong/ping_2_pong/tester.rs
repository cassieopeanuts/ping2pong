#[cfg(test)]
mod tests {
    use super::*;
    use hdk::prelude::*;
    use holochain_wasm_test_utils::TestWasm;

    #[test]
    fn test_create_player_unique_name() {
        // Initialize test environment
        let mut mock_env = TestWasm::from_zomes(vec![your_zome!()]);

        // Create first player
        let player1 = Player {
            player_key: AgentPubKey::from_raw_32([1; 32]),
            player_name: "Alice".into(),
        };
        let create_player_result = create_player(player1.clone());
        assert!(create_player_result.is_ok());

        // Attempt to create second player with the same name
        let player2 = Player {
            player_key: AgentPubKey::from_raw_32([2; 32]),
            player_name: "Alice".into(),
        };
        let create_player_result2 = create_player(player2.clone());
        assert!(create_player_result2.is_err());

        // Verify error message
        if let Err(wasm_error) = create_player_result2 {
            assert_eq!(
                wasm_error.0,
                WasmErrorInner::Guest("Player name must be unique".into())
            );
        }
    }

    #[test]
    fn test_create_game_with_invalid_players() {
        // Initialize test environment
        let mut mock_env = TestWasm::from_zomes(vec![your_zome!()]);

        // Create a player
        let player1 = Player {
            player_key: AgentPubKey::from_raw_32([1; 32]),
            player_name: "Alice".into(),
        };
        let _ = create_player(player1.clone());

        // Attempt to create a game with one valid player and one invalid player
        let game = Game {
            game_id: "game123".into(),
            player_1: player1.player_key.clone(),
            player_2: AgentPubKey::from_raw_32([3; 32]), // Not registered
            created_at: sys_time_now()?.into(),
            game_status: GameStatus::Waiting,
        };

        let create_game_result = create_game(game);
        assert!(create_game_result.is_err());

        // Verify error message
        if let Err(wasm_error) = create_game_result {
            assert_eq!(
                wasm_error.0,
                WasmErrorInner::Guest("Player 2 is not a registered player".into())
            );
        }
    }

    // Additional tests for updating games, deleting players, etc.
}

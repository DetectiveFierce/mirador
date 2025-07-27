//! Upgrade system for the Mirador game.
//!
//! This module provides a comprehensive upgrade system that allows players to enhance their
//! abilities throughout the game. Upgrades are categorized by rarity and can be randomly
//! selected based on weighted probabilities.
//!
//! # Overview
//!
//! The upgrade system consists of:
//! - `Upgrade`: Represents a single upgrade with name, tooltip, and rarity
//! - `UpgradeRarity`: Defines the rarity levels and their associated weights
//! - `AvailableUpgrade`: Enum of all possible upgrades in the game
//! - `UpgradeManager`: Manages player upgrades and provides selection logic
//!
//! # Usage
//!
//! ```rust
//! use mirador::game::upgrades::{UpgradeManager, AvailableUpgrade};
//!
//! let mut manager = UpgradeManager::new();
//!
//! // Select 3 random upgrades for the player to choose from
//! let options = manager.select_random_upgrades(3);
//!
//! // Apply a chosen upgrade
//! manager.apply_upgrade(&AvailableUpgrade::SpeedUp);
//!
//! // Check how many of a specific upgrade the player has
//! let speed_count = manager.get_upgrade_count(&AvailableUpgrade::SpeedUp);
//! ```

use rand::Rng;
use std::collections::HashMap;

/// Represents a single upgrade that can be applied to the player.
///
/// Each upgrade has a name, descriptive tooltip, and rarity level that determines
/// how likely it is to appear in random selections.
#[derive(Debug, Clone)]
pub struct Upgrade {
    /// The display name of the upgrade
    pub name: String,
    /// A description of what the upgrade does
    pub tooltip: String,
    /// The rarity level of the upgrade, affecting selection probability
    pub rarity: UpgradeRarity,
}

/// Defines the rarity levels for upgrades and their associated weights.
///
/// Rarer upgrades have lower weights, making them less likely to appear
/// in random selections. The weights are used for weighted random selection.
#[derive(Debug, Clone, PartialEq)]
pub enum UpgradeRarity {
    /// Common upgrades (40% chance)
    Common,
    /// Uncommon upgrades (30% chance)
    Uncommon,
    /// Rare upgrades (20% chance)
    Rare,
    /// Epic upgrades (8% chance)
    Epic,
    /// Legendary upgrades (2% chance)
    Legendary,
}

impl UpgradeRarity {
    /// Returns the weight value for this rarity level.
    ///
    /// Weights are used in weighted random selection. Higher weights
    /// mean the upgrade is more likely to be selected.
    ///
    /// # Returns
    ///
    /// A `f32` representing the weight of this rarity level.
    ///
    /// # Example
    ///
    /// ```rust
    /// use mirador::game::upgrades::UpgradeRarity;
    ///
    /// assert_eq!(UpgradeRarity::Common.weight(), 40.0);
    /// assert_eq!(UpgradeRarity::Legendary.weight(), 2.0);
    /// ```
    pub fn weight(&self) -> f32 {
        match self {
            UpgradeRarity::Common => 40.0,   // 40% chance
            UpgradeRarity::Uncommon => 30.0, // 30% chance
            UpgradeRarity::Rare => 20.0,     // 20% chance
            UpgradeRarity::Epic => 8.0,      // 8% chance
            UpgradeRarity::Legendary => 2.0, // 2% chance
        }
    }
}

/// Enum representing all available upgrades in the game.
///
/// Each variant corresponds to a specific upgrade that can be applied
/// to the player. This enum is used as a key in the upgrade manager
/// to track how many of each upgrade the player has collected.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AvailableUpgrade {
    /// Increases player movement speed
    SpeedUp,
    /// Makes time pass slower, giving more time to navigate
    SlowTime,
    /// Reduces noise made while moving
    SilentStep,
    /// Makes the player taller to see over walls
    TallBoots,
    /// Prevents enemy movement at level start
    HeadStart,
    /// Increases maximum stamina for longer sprinting
    Dash,
    /// A mysterious upgrade with unknown effects
    Unknown,
}

impl AvailableUpgrade {
    /// Converts this upgrade variant into a full `Upgrade` struct.
    ///
    /// This method provides the name, tooltip, and rarity for each upgrade type.
    ///
    /// # Returns
    ///
    /// A complete `Upgrade` struct with all information populated.
    ///
    /// # Example
    ///
    /// ```rust
    /// use mirador::game::upgrades::{AvailableUpgrade, UpgradeRarity};
    ///
    /// let upgrade = AvailableUpgrade::SpeedUp.to_upgrade();
    /// assert_eq!(upgrade.name, "Speed Up");
    /// assert_eq!(upgrade.rarity, UpgradeRarity::Common);
    /// ```
    pub fn to_upgrade(&self) -> Upgrade {
        match self {
            AvailableUpgrade::SpeedUp => Upgrade {
                name: "Speed Up".to_string(),
                tooltip: "Increases your movement speed, making you faster and more agile."
                    .to_string(),
                rarity: UpgradeRarity::Common,
            },
            AvailableUpgrade::SlowTime => Upgrade {
                name: "Slow Time".to_string(),
                tooltip: "Each second lasts longer, giving you more time to navigate the maze."
                    .to_string(),
                rarity: UpgradeRarity::Uncommon,
            },
            AvailableUpgrade::SilentStep => Upgrade {
                name: "Silent Step".to_string(),
                tooltip: "Reduces the noise you make while moving, making you harder to detect."
                    .to_string(),
                rarity: UpgradeRarity::Rare,
            },
            AvailableUpgrade::TallBoots => Upgrade {
                name: "Tall Boots".to_string(),
                tooltip: "Makes you taller, allowing you to better see over the walls of the maze."
                    .to_string(),
                rarity: UpgradeRarity::Uncommon,
            },
            AvailableUpgrade::HeadStart => Upgrade {
                name: "Head Start".to_string(),
                tooltip:
                    "Prevents the enemy from moving for a short time at the start of each level."
                        .to_string(),
                rarity: UpgradeRarity::Rare,
            },
            AvailableUpgrade::Dash => Upgrade {
                name: "Dash".to_string(),
                tooltip: "Increase your maximum stamina, allowing you to sprint for longer."
                    .to_string(),
                rarity: UpgradeRarity::Epic,
            },
            AvailableUpgrade::Unknown => Upgrade {
                name: "Unknown".to_string(),
                tooltip: "A mysterious upgrade with unpredictable effects. What could it do?"
                    .to_string(),
                rarity: UpgradeRarity::Legendary,
            },
        }
    }
}

/// Manages the player's upgrades and provides functionality for upgrade selection.
///
/// The `UpgradeManager` tracks how many of each upgrade the player has collected
/// and provides methods for selecting random upgrades and applying new ones.
///
/// # Examples
///
/// ```rust
/// use mirador::game::upgrades::{UpgradeManager, AvailableUpgrade};
///
/// let mut manager = UpgradeManager::new();
///
/// // Select 3 random upgrades for the player to choose from
/// let options = manager.select_random_upgrades(3);
///
/// // Apply a chosen upgrade
/// manager.apply_upgrade(&AvailableUpgrade::SpeedUp);
///
/// // Check upgrade count
/// assert_eq!(manager.get_upgrade_count(&AvailableUpgrade::SpeedUp), 1);
/// ```
pub struct UpgradeManager {
    /// Maps each upgrade type to the number of times the player has collected it
    pub player_upgrades: HashMap<AvailableUpgrade, u32>, // upgrade -> count
}

impl UpgradeManager {
    /// Creates a new `UpgradeManager` with no upgrades collected.
    ///
    /// # Returns
    ///
    /// A new `UpgradeManager` instance with an empty upgrade collection.
    pub fn new() -> Self {
        Self {
            player_upgrades: HashMap::new(),
        }
    }

    /// Selects a specified number of random upgrades for the player to choose from.
    ///
    /// This method uses weighted random selection based on upgrade rarity to ensure
    /// that rarer upgrades appear less frequently. No duplicate upgrades are returned
    /// in a single selection.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of upgrades to select
    ///
    /// # Returns
    ///
    /// A vector of `Upgrade` structs. If `count` is greater than the number of
    /// available upgrades, fewer upgrades than requested may be returned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mirador::game::upgrades::UpgradeManager;
    ///
    /// let manager = UpgradeManager::new();
    /// let selected = manager.select_random_upgrades(3);
    ///
    /// // Should get exactly 3 unique upgrades
    /// assert_eq!(selected.len(), 3);
    ///
    /// // All upgrades should be unique
    /// let names: Vec<String> = selected.iter().map(|u| u.name.clone()).collect();
    /// let unique_names: Vec<String> = names.iter().cloned().collect::<std::collections::HashSet<_>>().into_iter().collect();
    /// assert_eq!(names.len(), unique_names.len());
    /// ```
    pub fn select_random_upgrades(&self, count: usize) -> Vec<Upgrade> {
        let mut rng = rand::thread_rng();
        let mut selected_upgrades = Vec::new();

        // Get all available upgrades
        let mut available_upgrades = vec![
            AvailableUpgrade::SpeedUp,
            AvailableUpgrade::SlowTime,
            AvailableUpgrade::SilentStep,
            AvailableUpgrade::TallBoots,
            AvailableUpgrade::HeadStart,
            AvailableUpgrade::Dash,
        ];

        // Weighted random selection based on rarity, ensuring no duplicates
        for _ in 0..count {
            // If we've run out of upgrades, break to avoid infinite loop
            // This can happen if count > available_upgrades.len()
            if available_upgrades.is_empty() {
                break;
            }

            // Calculate total weight of remaining upgrades
            let total_weight: f32 = available_upgrades
                .iter()
                .map(|upgrade| upgrade.to_upgrade().rarity.weight())
                .sum();

            // Generate random value
            let random_value = rng.gen_range(0.0..total_weight);

            // Find the upgrade based on weight
            let mut cumulative_weight = 0.0;
            let mut selected_index = 0; // Default to first upgrade

            for (index, upgrade) in available_upgrades.iter().enumerate() {
                cumulative_weight += upgrade.to_upgrade().rarity.weight();
                if cumulative_weight >= random_value {
                    selected_index = index;
                    break;
                }
            }

            // Get the selected upgrade and remove it from available pool
            let selected_upgrade = available_upgrades.remove(selected_index);

            // Convert to Upgrade and add to selection
            selected_upgrades.push(selected_upgrade.to_upgrade());
        }

        // Note: If count > available_upgrades.len(), this will return fewer upgrades than requested
        // This is the expected behavior to ensure no duplicates
        selected_upgrades
    }

    /// Applies an upgrade to the player, incrementing its count.
    ///
    /// This method increases the count of the specified upgrade by 1.
    /// If the upgrade hasn't been collected before, it will be added
    /// to the collection with a count of 1.
    ///
    /// # Arguments
    ///
    /// * `upgrade` - The upgrade to apply
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mirador::game::upgrades::{UpgradeManager, AvailableUpgrade};
    ///
    /// let mut manager = UpgradeManager::new();
    ///
    /// // Apply an upgrade for the first time
    /// manager.apply_upgrade(&AvailableUpgrade::SpeedUp);
    /// assert_eq!(manager.get_upgrade_count(&AvailableUpgrade::SpeedUp), 1);
    ///
    /// // Apply the same upgrade again
    /// manager.apply_upgrade(&AvailableUpgrade::SpeedUp);
    /// assert_eq!(manager.get_upgrade_count(&AvailableUpgrade::SpeedUp), 2);
    /// ```
    pub fn apply_upgrade(&mut self, upgrade: &AvailableUpgrade) {
        let current_count = *self.player_upgrades.get(upgrade).unwrap_or(&0);
        self.player_upgrades
            .insert(upgrade.clone(), current_count + 1);

        let upgrade_info = upgrade.to_upgrade();
        println!(
            "Applied upgrade: {} (Total: {})",
            upgrade_info.name,
            current_count + 1
        );
    }

    /// Returns the number of times the player has collected a specific upgrade.
    ///
    /// # Arguments
    ///
    /// * `upgrade` - The upgrade to check
    ///
    /// # Returns
    ///
    /// The number of times the upgrade has been collected, or 0 if never collected.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mirador::game::upgrades::{UpgradeManager, AvailableUpgrade};
    ///
    /// let mut manager = UpgradeManager::new();
    ///
    /// // Initially no upgrades collected
    /// assert_eq!(manager.get_upgrade_count(&AvailableUpgrade::SpeedUp), 0);
    ///
    /// // After applying an upgrade
    /// manager.apply_upgrade(&AvailableUpgrade::SpeedUp);
    /// assert_eq!(manager.get_upgrade_count(&AvailableUpgrade::SpeedUp), 1);
    /// ```
    pub fn get_upgrade_count(&self, upgrade: &AvailableUpgrade) -> u32 {
        *self.player_upgrades.get(upgrade).unwrap_or(&0)
    }

    /// Returns display information for an upgrade, including level and tooltip.
    ///
    /// This method is useful for UI display, providing both the current level
    /// of the upgrade (if collected) and its tooltip description.
    ///
    /// # Arguments
    ///
    /// * `upgrade` - The upgrade to get display info for
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - `String`: The level text (e.g., "Level 2" or "New Upgrade")
    /// - `String`: The tooltip description
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mirador::game::upgrades::{UpgradeManager, AvailableUpgrade};
    ///
    /// let mut manager = UpgradeManager::new();
    /// let upgrade = AvailableUpgrade::SpeedUp.to_upgrade();
    ///
    /// // For a new upgrade
    /// let (level, tooltip) = manager.get_upgrade_display_info(&upgrade);
    /// assert_eq!(level, "New Upgrade");
    /// assert!(tooltip.contains("movement speed"));
    ///
    /// // After applying the upgrade
    /// manager.apply_upgrade(&AvailableUpgrade::SpeedUp);
    /// let (level, tooltip) = manager.get_upgrade_display_info(&upgrade);
    /// assert_eq!(level, "Level 1");
    /// ```
    pub fn get_upgrade_display_info(&self, upgrade: &Upgrade) -> (String, String) {
        // Find the corresponding AvailableUpgrade
        let available_upgrade = self.find_available_upgrade(upgrade);
        let current_count = self.get_upgrade_count(&available_upgrade);

        let level_text = if current_count > 0 {
            format!("Level {}", current_count)
        } else {
            "New Upgrade".to_string()
        };

        let tooltip_text = upgrade.tooltip.clone();

        (level_text, tooltip_text)
    }

    /// Finds the corresponding `AvailableUpgrade` for a given `Upgrade` struct.
    ///
    /// This is a helper method that maps from the display `Upgrade` back to
    /// the enum variant used internally for tracking.
    ///
    /// # Arguments
    ///
    /// * `upgrade` - The upgrade to find the enum variant for
    ///
    /// # Returns
    ///
    /// The corresponding `AvailableUpgrade` enum variant, or `SpeedUp` as fallback.
    fn find_available_upgrade(&self, upgrade: &Upgrade) -> AvailableUpgrade {
        // Find the AvailableUpgrade that matches this Upgrade
        let all_upgrades = vec![
            AvailableUpgrade::SpeedUp,
            AvailableUpgrade::SlowTime,
            AvailableUpgrade::SilentStep,
            AvailableUpgrade::TallBoots,
            AvailableUpgrade::HeadStart,
            AvailableUpgrade::Dash,
        ];

        for available in all_upgrades {
            if available.to_upgrade().name == upgrade.name {
                return available;
            }
        }

        // Fallback
        AvailableUpgrade::SpeedUp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that no duplicate upgrades are returned in a single selection.
    #[test]
    fn test_no_duplicates_in_selection() {
        let upgrade_manager = UpgradeManager::new();

        // Test multiple selections to ensure no duplicates
        for _ in 0..10 {
            let selected = upgrade_manager.select_random_upgrades(3);

            // Check that we got exactly 3 upgrades
            assert_eq!(selected.len(), 3);

            // Check that all upgrades are unique
            let mut names: Vec<String> = selected.iter().map(|u| u.name.clone()).collect();
            names.sort();
            names.dedup(); // Remove duplicates
            assert_eq!(names.len(), 3, "Found duplicate upgrades: {:?}", selected);
        }
    }

    /// Tests that all available upgrades can be selected when requesting enough.
    #[test]
    fn test_all_upgrades_available() {
        let upgrade_manager = UpgradeManager::new();
        let selected = upgrade_manager.select_random_upgrades(6);

        // Should get all 6 upgrades when requesting 6
        assert_eq!(selected.len(), 6);

        // All should be unique
        let mut names: Vec<String> = selected.iter().map(|u| u.name.clone()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 6);
    }
}

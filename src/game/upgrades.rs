use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Upgrade {
    pub name: String,
    pub tooltip: String,
    pub rarity: UpgradeRarity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpgradeRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl UpgradeRarity {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AvailableUpgrade {
    SpeedUp,
    SlowTime,
    SilentStep,
    TallBoots,
    HeadStart,
    Dash,
    Unknown,
}

impl AvailableUpgrade {
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

pub struct UpgradeManager {
    pub player_upgrades: HashMap<AvailableUpgrade, u32>, // upgrade -> count
}

impl UpgradeManager {
    pub fn new() -> Self {
        Self {
            player_upgrades: HashMap::new(),
        }
    }

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

    pub fn get_upgrade_count(&self, upgrade: &AvailableUpgrade) -> u32 {
        *self.player_upgrades.get(upgrade).unwrap_or(&0)
    }

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

    #[test]
    fn test_all_upgrades_available() {
        let upgrade_manager = UpgradeManager::new();
        let selected = upgrade_manager.select_random_upgrades(7);

        // Should get all 7 upgrades when requesting 7
        assert_eq!(selected.len(), 7);

        // All should be unique
        let mut names: Vec<String> = selected.iter().map(|u| u.name.clone()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 7);
    }
}

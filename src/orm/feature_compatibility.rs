/// Feature compatibility checker for advanced SQL features across different database systems
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::orm::advanced_sql::*;
use crate::orm::{SqlResult, SqlError};

/// Feature compatibility report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub database: String,
    pub version: DatabaseVersion,
    pub features: HashMap<String, FeatureCompatibility>,
    pub overall_score: f64,
    pub recommendations: Vec<String>,
}

/// Individual feature compatibility information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureCompatibility {
    pub feature_name: String,
    pub supported: bool,
    pub minimum_version: Option<DatabaseVersion>,
    pub current_version_compatible: bool,
    pub notes: Option<String>,
    pub alternatives: Vec<String>,
}

/// Feature compatibility checker
pub struct FeatureCompatibilityChecker {
    plugins: HashMap<String, Box<dyn AdvancedSqlPlugin + Send + Sync>>,
}

impl FeatureCompatibilityChecker {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }
    
    /// Register a database plugin
    pub fn register_plugin(&mut self, name: String, plugin: Box<dyn AdvancedSqlPlugin + Send + Sync>) {
        self.plugins.insert(name, plugin);
    }
    
    /// Check compatibility for a specific database
    pub fn check_compatibility(&self, database_name: &str) -> SqlResult<CompatibilityReport> {
        let plugin = self.plugins.get(database_name)
            .ok_or_else(|| SqlError::runtime_error(&format!("Unknown database: {}", database_name)))?;
        
        let (db_name, version) = plugin.database_info();
        let mut features = HashMap::new();
        let mut recommendations = Vec::new();
        
        // Check CTE support
        let cte_support = plugin.supports_cte();
        features.insert("Common Table Expressions (CTE)".to_string(), FeatureCompatibility {
            feature_name: "Common Table Expressions (CTE)".to_string(),
            supported: cte_support.supported,
            minimum_version: cte_support.minimum_version.clone(),
            current_version_compatible: check_version_compatibility(&version, &cte_support),
            notes: cte_support.notes.clone(),
            alternatives: if !cte_support.supported {
                vec!["Use subqueries or temporary tables".to_string()]
            } else {
                vec![]
            },
        });
        
        // Check recursive CTE support
        let recursive_cte_support = plugin.supports_recursive_cte();
        features.insert("Recursive CTEs".to_string(), FeatureCompatibility {
            feature_name: "Recursive CTEs".to_string(),
            supported: recursive_cte_support.supported,
            minimum_version: recursive_cte_support.minimum_version.clone(),
            current_version_compatible: check_version_compatibility(&version, &recursive_cte_support),
            notes: recursive_cte_support.notes.clone(),
            alternatives: if !recursive_cte_support.supported {
                vec!["Use iterative queries with loops".to_string(), "Use application-level recursion".to_string()]
            } else {
                vec![]
            },
        });
        
        // Check window functions support
        let window_support = plugin.supports_window_functions();
        features.insert("Window Functions".to_string(), FeatureCompatibility {
            feature_name: "Window Functions".to_string(),
            supported: window_support.supported,
            minimum_version: window_support.minimum_version.clone(),
            current_version_compatible: check_version_compatibility(&version, &window_support),
            notes: window_support.notes.clone(),
            alternatives: if !window_support.supported {
                vec!["Use self-joins with subqueries".to_string(), "Use correlated subqueries".to_string()]
            } else {
                vec![]
            },
        });
        
        // Check JSON support
        let json_support = plugin.supports_json();
        features.insert("JSON Operations".to_string(), FeatureCompatibility {
            feature_name: "JSON Operations".to_string(),
            supported: json_support.supported,
            minimum_version: json_support.minimum_version.clone(),
            current_version_compatible: check_version_compatibility(&version, &json_support),
            notes: json_support.notes.clone(),
            alternatives: if !json_support.supported {
                vec!["Store JSON as TEXT and parse in application".to_string(), "Use separate tables for structured data".to_string()]
            } else {
                vec![]
            },
        });
        
        // Check full-text search support
        let fts_support = plugin.supports_full_text_search();
        features.insert("Full-Text Search".to_string(), FeatureCompatibility {
            feature_name: "Full-Text Search".to_string(),
            supported: fts_support.supported,
            minimum_version: fts_support.minimum_version.clone(),
            current_version_compatible: check_version_compatibility(&version, &fts_support),
            notes: fts_support.notes.clone(),
            alternatives: if !fts_support.supported {
                vec!["Use LIKE queries with indexes".to_string(), "Use external search engines (Elasticsearch, Solr)".to_string()]
            } else {
                vec![]
            },
        });
        
        // Calculate overall compatibility score
        let total_features = features.len() as f64;
        let compatible_features = features.values()
            .filter(|f| f.current_version_compatible)
            .count() as f64;
        let overall_score = (compatible_features / total_features) * 100.0;
        
        // Generate recommendations
        if overall_score < 80.0 {
            recommendations.push("Consider upgrading to a newer database version for better feature support".to_string());
        }
        
        for feature in features.values() {
            if !feature.current_version_compatible && feature.supported {
                if let Some(ref min_version) = feature.minimum_version {
                    recommendations.push(format!(
                        "Upgrade to {} {} or later to use {}",
                        db_name, min_version, feature.feature_name
                    ));
                }
            }
        }
        
        Ok(CompatibilityReport {
            database: db_name,
            version,
            features,
            overall_score,
            recommendations,
        })
    }
    
    /// Compare compatibility across multiple databases
    pub fn compare_databases(&self, database_names: &[String]) -> SqlResult<Vec<CompatibilityReport>> {
        let mut reports = Vec::new();
        
        for db_name in database_names {
            reports.push(self.check_compatibility(db_name)?);
        }
        
        Ok(reports)
    }
    
    /// Get feature support matrix across all registered databases
    pub fn get_feature_matrix(&self) -> SqlResult<FeatureMatrix> {
        let mut matrix = FeatureMatrix::new();
        
        for (db_name, plugin) in &self.plugins {
            let (_, version) = plugin.database_info();
            
            let features = vec![
                ("CTE", plugin.supports_cte()),
                ("Recursive CTE", plugin.supports_recursive_cte()),
                ("Window Functions", plugin.supports_window_functions()),
                ("JSON Operations", plugin.supports_json()),
                ("Full-Text Search", plugin.supports_full_text_search()),
            ];
            
            for (feature_name, support) in features {
                matrix.add_support(
                    feature_name.to_string(),
                    db_name.clone(),
                    version.clone(),
                    support,
                );
            }
        }
        
        Ok(matrix)
    }
    
    /// Recommend the best database for a set of required features
    pub fn recommend_database(&self, required_features: &[String]) -> SqlResult<DatabaseRecommendation> {
        let mut scores = HashMap::new();
        
        for (db_name, plugin) in &self.plugins {
            let (_, version) = plugin.database_info();
            let mut score = 0.0;
            let mut supported_features = Vec::new();
            let mut unsupported_features = Vec::new();
            
            for feature in required_features {
                let support = match feature.as_str() {
                    "CTE" => plugin.supports_cte(),
                    "Recursive CTE" => plugin.supports_recursive_cte(),
                    "Window Functions" => plugin.supports_window_functions(),
                    "JSON Operations" => plugin.supports_json(),
                    "Full-Text Search" => plugin.supports_full_text_search(),
                    _ => FeatureSupport::not_supported(),
                };
                
                if support.supported && check_version_compatibility(&version, &support) {
                    score += 1.0;
                    supported_features.push(feature.clone());
                } else {
                    unsupported_features.push(feature.clone());
                }
            }
            
            scores.insert(db_name.clone(), DatabaseScore {
                database: db_name.clone(),
                version: version.clone(),
                score: score / required_features.len() as f64,
                supported_features,
                unsupported_features,
            });
        }
        
        let best_match = scores.values()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
            .cloned();
        
        Ok(DatabaseRecommendation {
            required_features: required_features.to_vec(),
            database_scores: scores.into_values().collect(),
            recommended: best_match,
        })
    }
}

/// Feature support matrix across databases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureMatrix {
    pub features: HashMap<String, HashMap<String, DatabaseFeatureInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseFeatureInfo {
    pub database: String,
    pub version: DatabaseVersion,
    pub support: FeatureSupport,
}

impl FeatureMatrix {
    pub fn new() -> Self {
        Self {
            features: HashMap::new(),
        }
    }
    
    pub fn add_support(
        &mut self,
        feature: String,
        database: String,
        version: DatabaseVersion,
        support: FeatureSupport,
    ) {
        self.features
            .entry(feature)
            .or_insert_with(HashMap::new)
            .insert(database.clone(), DatabaseFeatureInfo {
                database,
                version,
                support,
            });
    }
}

/// Database recommendation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseRecommendation {
    pub required_features: Vec<String>,
    pub database_scores: Vec<DatabaseScore>,
    pub recommended: Option<DatabaseScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseScore {
    pub database: String,
    pub version: DatabaseVersion,
    pub score: f64,
    pub supported_features: Vec<String>,
    pub unsupported_features: Vec<String>,
}

// Helper function to check version compatibility
fn check_version_compatibility(current: &DatabaseVersion, support: &FeatureSupport) -> bool {
    if !support.supported {
        return false;
    }
    
    if let Some(ref min_version) = support.minimum_version {
        current.is_at_least(min_version)
    } else {
        true
    }
}

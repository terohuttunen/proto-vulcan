use colored::*;
use std::time::Instant;

/// Progressive tracing configuration (immutable settings)
#[derive(Clone, Debug)]
pub struct TraceConfig {
    pub enabled: bool,
    pub level: TraceLevel,
}

/// Runtime state for trace execution (mutable state)
#[derive(Debug)]
pub struct TraceState {
    // Store config copy in state
    pub enabled: bool,
    pub level: TraceLevel,
    // Rest of the existing fields
    pub start_time: Instant,
    current_depth: usize,
    choice_point_counter: usize,
    branch_stack: Vec<BranchInfo>,
    relation_call_stack: Vec<String>,
    solutions_found: usize,
    recursion_patterns: Vec<RecursionPattern>,
    in_recursion: bool,
    current_recursion_depth: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TraceLevel {
    /// Level 1: Basic summary only  
    Basic = 1,
    /// Level 2: Shows choice points and branches
    Medium = 2,
    /// Level 3: Detailed tracing with unification steps
    Detailed = 3,
}

impl TraceLevel {
    /// Convert a numeric level to TraceLevel enum
    pub fn from_u8(level: u8) -> Self {
        match level {
            1 => TraceLevel::Basic,
            3 => TraceLevel::Detailed,
            _ => TraceLevel::Medium, // Default to 2
        }
    }
}

#[derive(Clone, Debug)]
struct BranchInfo {
    choice_point_id: usize,
    branch_number: usize,
    total_branches: usize,
    relation_name: String,
    depth: usize,
}

#[derive(Clone, Debug)]
struct RecursionPattern {
    relation_name: String,
    start_depth: usize,
    call_chain: Vec<String>,
    max_depth: usize,
    is_collapsed: bool,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            level: TraceLevel::Medium,
        }
    }
}

impl TraceConfig {
    pub fn new(enabled: bool, level: u8) -> Self {
        Self {
            enabled,
            level: TraceLevel::from_u8(level),
        }
    }
}

impl TraceState {
    pub fn new(config: TraceConfig) -> Self {
        Self {
            enabled: config.enabled,
            level: config.level,
            start_time: Instant::now(),
            current_depth: 0,
            choice_point_counter: 0,
            branch_stack: Vec::new(),
            relation_call_stack: Vec::new(),
            solutions_found: 0,
            recursion_patterns: Vec::new(),
            in_recursion: false,
            current_recursion_depth: 0,
        }
    }

    /// Print the initial search trace header
    pub fn print_search_header(&self) {
        if !self.enabled {
            return;
        }

        // All trace levels show the search trace header
        println!("{}:", "Search Trace".bright_blue().bold());
    }

    /// Enter a new relation call
    pub fn enter_relation(&mut self, name: &str) {
        if !self.enabled {
            return;
        }

        self.current_depth += 1;
        self.relation_call_stack.push(name.to_string());

        // Check for recursion
        let is_recursive_call = self.is_recursive_call(name);

        if is_recursive_call && !self.in_recursion {
            // Starting a new recursion
            self.start_recursion_pattern(name);
        } else if is_recursive_call && self.in_recursion {
            // Continuing recursion - update the pattern
            self.extend_recursion_pattern(name);
        } else if self.in_recursion && !is_recursive_call {
            // Recursion ended, show the pattern
            self.end_recursion_pattern();
        }

        match self.level {
            TraceLevel::Basic => {
                // Basic level doesn't show individual relation calls during execution
            }
            TraceLevel::Medium => {
                if self.current_depth == 1 {
                    // Show top-level relation entry (header already printed in main.rs)
                    println!("  {} {}", "→".bright_white(), name.bright_white());
                } else if !self.in_recursion {
                    // Show non-recursive calls in medium detail
                    let indent = self.get_medium_indent();
                    println!("{}→ {}", indent, name.bright_white());
                }
            }
            TraceLevel::Detailed => {
                if !self.in_recursion {
                    let indent = self.get_detailed_indent();
                    println!(
                        "{}→ {} {} {}",
                        indent,
                        name.bright_white(),
                        "at depth".white(),
                        self.current_depth.to_string().bright_yellow()
                    );
                }
            }
        }
    }

    /// Exit a relation call
    pub fn exit_relation(&mut self, name: &str, completed: bool) {
        if !self.enabled {
            return;
        }

        if let Some(last_rel) = self.relation_call_stack.pop() {
            if last_rel != name {
                // Stack mismatch - this shouldn't happen but let's be defensive
                self.relation_call_stack.push(last_rel);
            }
        }

        // Check if we're exiting from a recursion
        if self.in_recursion && !self.is_recursive_call(name) {
            self.end_recursion_pattern();
        }

        match self.level {
            TraceLevel::Basic => {
                // Basic level doesn't show exits during execution
            }
            TraceLevel::Medium => {
                if !self.in_recursion {
                    // Only show non-recursive exits to avoid clutter
                    if self.current_depth <= 2 {
                        let indent = self.get_medium_indent();
                        let status = if completed {
                            "completed".bright_green()
                        } else {
                            "failed".bright_red()
                        };
                        println!("{}← {}: {}", indent, name.bright_white(), status);
                    }
                }
            }
            TraceLevel::Detailed => {
                if !self.in_recursion {
                    let indent = self.get_detailed_indent();
                    let status = if completed {
                        "Completed".bright_green()
                    } else {
                        "Failed".bright_red()
                    };
                    println!("{}← {}: {}", indent, name.bright_white(), status);
                }
            }
        }

        self.current_depth = self.current_depth.saturating_sub(1);
    }

    /// Enter a choice point with multiple branches
    pub fn enter_choice_point(&mut self, relation_name: &str, branch_count: usize) {
        if !self.enabled || branch_count <= 1 {
            return;
        }

        self.choice_point_counter += 1;

        match self.level {
            TraceLevel::Basic => {
                // Basic level doesn't show choice points during execution
            }
            TraceLevel::Medium => {
                let indent = self.get_medium_indent();
                println!(
                    "{}Choice point: {} {}",
                    indent,
                    branch_count.to_string().bright_yellow(),
                    "branches".white()
                );
            }
            TraceLevel::Detailed => {
                let indent = self.get_detailed_indent();
                println!(
                    "{}→ Choice point #{}: {} {} available",
                    indent,
                    self.choice_point_counter.to_string().bright_cyan(),
                    branch_count.to_string().bright_yellow(),
                    "branches".white()
                );
            }
        }
    }

    /// Enter a specific branch of a choice point
    pub fn enter_branch(&mut self, branch_number: usize, total_branches: usize, description: &str) {
        if !self.enabled {
            return;
        }

        let branch_info = BranchInfo {
            choice_point_id: self.choice_point_counter,
            branch_number,
            total_branches,
            relation_name: description.to_string(),
            depth: self.current_depth,
        };
        self.branch_stack.push(branch_info.clone());

        match self.level {
            TraceLevel::Basic => {
                // Basic level doesn't show branches during execution
            }
            TraceLevel::Medium => {
                let indent = self.get_medium_indent();
                if branch_number <= total_branches {
                    println!(
                        "{}→ Branch {}: {}",
                        indent,
                        branch_number.to_string().bright_yellow(),
                        description.bright_white()
                    );
                }
            }
            TraceLevel::Detailed => {
                let indent = self.get_detailed_indent();
                println!(
                    "{}  → Branch {}/{}: {}",
                    indent,
                    branch_number.to_string().bright_yellow(),
                    total_branches.to_string().bright_cyan(),
                    description.bright_white()
                );
            }
        }
    }

    /// Exit a branch with success or failure
    pub fn exit_branch(&mut self, succeeded: bool) {
        if !self.enabled {
            return;
        }

        if let Some(branch_info) = self.branch_stack.pop() {
            match self.level {
                TraceLevel::Basic => {
                    // Basic level doesn't show branch exits during execution
                }
                TraceLevel::Medium => {
                    let indent = self.get_medium_indent();
                    if succeeded {
                        println!(
                            "{}✓ Branch {}: {}",
                            indent,
                            branch_info.branch_number.to_string().bright_yellow(),
                            "Success".bright_green()
                        );
                    }
                    // Don't clutter medium level with failures
                }
                TraceLevel::Detailed => {
                    let indent = self.get_detailed_indent();
                    let status = if succeeded {
                        "Success".bright_green()
                    } else {
                        "Failed".bright_red()
                    };
                    println!(
                        "{}  ← Branch {}/{}: {}",
                        indent,
                        branch_info.branch_number.to_string().bright_yellow(),
                        branch_info.total_branches.to_string().bright_cyan(),
                        status
                    );
                }
            }
        }
    }

    /// Record a solution found
    pub fn trace_solution(&mut self, bindings: &[(String, String)]) {
        if !self.enabled {
            return;
        }

        self.solutions_found += 1;

        match self.level {
            TraceLevel::Basic => {
                // Basic level just counts solutions, shows them at the end
            }
            TraceLevel::Medium => {
                let indent = self.get_medium_indent();
                if bindings.is_empty() {
                    println!("{}✓ Solution: {}", indent, "true".bright_green());
                } else {
                    let binding_str = bindings
                        .iter()
                        .map(|(var, val)| format!("{} {} {}", var.bright_cyan(), "=".white(), val))
                        .collect::<Vec<_>>()
                        .join(", ");
                    println!("{}✓ Solution: {}", indent, binding_str);
                }
            }
            TraceLevel::Detailed => {
                let indent = self.get_detailed_indent();
                println!(
                    "{}          ✓ Solution: {}",
                    indent,
                    if bindings.is_empty() {
                        "true".bright_green().to_string()
                    } else {
                        bindings
                            .iter()
                            .map(|(var, val)| {
                                format!("{} {} {}", var.bright_cyan(), "=".white(), val)
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                );
            }
        }
    }

    /// Show unification step (detailed level only)
    pub fn trace_unification(&mut self, var: &str, value: &str, description: &str) {
        if !self.enabled || self.level != TraceLevel::Detailed {
            return;
        }

        let indent = self.get_detailed_indent();
        println!(
            "{}            → {}: {} {} {}",
            indent,
            description.white(),
            var.bright_cyan(),
            "=".white(),
            value.bright_white()
        );
    }

    /// Print execution summary based on trace level
    pub fn print_summary(&self) {
        if !self.enabled {
            return;
        }

        match self.level {
            TraceLevel::Basic => {
                let _elapsed = self.start_time.elapsed();

                // Show recursion summary if any occurred
                if !self.recursion_patterns.is_empty() {
                    for pattern in &self.recursion_patterns {
                        println!(
                            "↻ recursion: {} {} {}",
                            pattern.relation_name.bright_cyan(),
                            "[max depth:".white(),
                            format!("{}]", pattern.max_depth).bright_yellow()
                        );
                    }
                }

                println!(
                    "{}: {} → {} {} found",
                    "Search".bright_blue().bold(),
                    if self.relation_call_stack.is_empty() {
                        "completed".bright_green()
                    } else {
                        "incomplete".bright_red()
                    },
                    self.solutions_found.to_string().bright_yellow(),
                    if self.solutions_found == 1 {
                        "solution"
                    } else {
                        "solutions"
                    }
                );
            }
            TraceLevel::Medium | TraceLevel::Detailed => {
                // These levels show progress during execution, no need for summary
            }
        }
    }

    /// Get indentation for medium detail level
    fn get_medium_indent(&self) -> String {
        "    ".repeat(self.current_depth.saturating_sub(1).min(4))
    }

    /// Get indentation for detailed level  
    fn get_detailed_indent(&self) -> String {
        "  ".repeat(self.current_depth.saturating_sub(1))
    }

    /// Check if current call is recursive
    fn is_recursive_call(&self, name: &str) -> bool {
        self.relation_call_stack.iter().any(|rel| rel == name)
    }

    /// Start tracking a new recursion pattern
    fn start_recursion_pattern(&mut self, name: &str) {
        self.in_recursion = true;
        self.current_recursion_depth = 1;

        let pattern = RecursionPattern {
            relation_name: name.to_string(),
            start_depth: self.current_depth,
            call_chain: vec![name.to_string()],
            max_depth: self.current_depth,
            is_collapsed: false,
        };

        self.recursion_patterns.push(pattern);

        // Show recursion start
        match self.level {
            TraceLevel::Basic => {
                // Basic level shows recursion summary later
            }
            TraceLevel::Medium => {
                let indent = self.get_medium_indent();
                println!("{}↻ recursion: {} [starting]", indent, name.bright_cyan());
            }
            TraceLevel::Detailed => {
                let indent = self.get_detailed_indent();
                println!(
                    "{}↻ recursion detected: {} {} {}",
                    indent,
                    name.bright_cyan(),
                    "at depth".white(),
                    self.current_depth.to_string().bright_yellow()
                );
            }
        }
    }

    /// Extend current recursion pattern
    fn extend_recursion_pattern(&mut self, name: &str) {
        self.current_recursion_depth += 1;

        if let Some(pattern) = self.recursion_patterns.last_mut() {
            pattern.call_chain.push(name.to_string());
            pattern.max_depth = self.current_depth;
        }
    }

    /// End current recursion pattern and show summary
    fn end_recursion_pattern(&mut self) {
        if let Some(pattern) = self.recursion_patterns.last() {
            let depth = self.current_recursion_depth;
            let chain_preview = if pattern.call_chain.len() <= 3 {
                pattern.call_chain.join("→")
            } else {
                format!(
                    "{}→...→{}",
                    pattern.call_chain[0],
                    pattern.call_chain.last().unwrap()
                )
            };

            match self.level {
                TraceLevel::Basic => {
                    // Will be shown in summary
                }
                TraceLevel::Medium => {
                    let indent = self.get_medium_indent();
                    println!(
                        "{}↻ recursion: {} {} {}",
                        indent,
                        chain_preview.bright_cyan(),
                        "[depth:".white(),
                        format!("{}]", depth).bright_yellow()
                    );
                }
                TraceLevel::Detailed => {
                    let indent = self.get_detailed_indent();
                    println!(
                        "{}↻ recursion chain: {} → {} {}",
                        indent,
                        pattern.relation_name.bright_cyan(),
                        chain_preview.bright_white(),
                        format!("[max depth: {}]", pattern.max_depth).bright_yellow()
                    );
                }
            }
        }

        self.in_recursion = false;
        self.current_recursion_depth = 0;
    }
}

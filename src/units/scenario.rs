extern crate dependy;
extern crate systemd_parser;

use std::path::Path;
use std::time::Duration;
use std::io::Read;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use self::systemd_parser::items::DirectiveEntry;
use self::dependy::{Dependy, Dependency};

use config::Config;
use unit::{UnitActivateError, UnitDeactivateError, UnitDescriptionError, UnitIncompatibleReason,
           UnitName};
use unitmanager::UnitManager;
use units::test::Test;

struct AssumptionDependency {
    name: String,
    requirements: Vec<String>,
    suggestions: Vec<String>,
    provides: Vec<String>,
}

impl AssumptionDependency {
    pub fn new(name: String) -> AssumptionDependency {
        AssumptionDependency {
            name: name,
            requirements: vec![],
            suggestions: vec![],
            provides: vec![],
        }
    }
}

impl Dependency for AssumptionDependency {
    fn name(&self) -> &str {
        &self.name.as_str()
    }
    fn requirements(&self) -> &Vec<String> {
        &self.requirements
    }
    fn suggestions(&self) -> &Vec<String> {
        &self.suggestions
    }
    fn provides(&self) -> &Vec<String> {
        &self.provides
    }
}

/// A struct defining an in-memory representation of a .scenario file
pub struct ScenarioDescription {
    /// The id of the unit (including the kind)
    id: UnitName,

    /// A short name
    name: String,

    /// A detailed description of this jig, up to one paragraph.
    description: String,

    /// A Vec<String> of jig names that this test is compatible with.
    jigs: Vec<UnitName>,

    /// A Vec<String> of test names that are explicitly specified.
    tests: Vec<UnitName>,

    /// A Vec<String> of tests that are considered to have passed without running them.
    assumptions: Vec<UnitName>,

    /// The maximum duration, if any, for this scenario
    timeout: Option<Duration>,

    /// A command to run when a scenario completes successfully.
    exec_stop_success: Option<String>,

    /// The maximum amount of time to allow the "success" script to run for.
    exec_stop_success_timeout: Option<Duration>,

    /// An optional command to run when the scenario does not complete successfully.
    exec_stop_failure: Option<String>,

    /// The maximum amount of time to allow the "failure" script to run for.
    exec_stop_failure_timeout: Option<Duration>,
}

impl ScenarioDescription {
    pub fn from_path(path: &Path) -> Result<ScenarioDescription, UnitDescriptionError> {
        let unit_name = UnitName::from_path(path)?;

        // Parse the file into a systemd unit_file object
        let mut contents = String::with_capacity(8192);
        File::open(path)?.read_to_string(&mut contents)?;
        let unit_file = systemd_parser::parse_string(&contents)?;

        if !unit_file.has_category("Scenario") {
            return Err(UnitDescriptionError::MissingSection("Scenario".to_owned()));
        }

        let mut scenario_description = ScenarioDescription {
            id: unit_name,
            name: "".to_owned(),
            description: "".to_owned(),

            jigs: vec![],
            tests: vec![],
            assumptions: vec![],

            timeout: None,

            exec_stop_success: None,
            exec_stop_success_timeout: None,
            exec_stop_failure: None,
            exec_stop_failure_timeout: None,
        };

        for entry in unit_file.lookup_by_category("Scenario") {
            match entry {
                &DirectiveEntry::Solo(ref directive) => {
                    match directive.key() {
                        "Name" => {
                            scenario_description.name = directive.value().unwrap_or("").to_owned()
                        }
                        "Description" => {
                            scenario_description.description =
                                directive.value().unwrap_or("").to_owned()
                        }
                        "Jigs" => {
                            scenario_description.jigs = match directive.value() {
                                Some(s) => UnitName::from_list(s, "jig")?,
                                None => vec![],
                            }
                        }
                        "Tests" => {
                            scenario_description.tests = match directive.value() {
                                Some(s) => UnitName::from_list(s, "test")?,
                                None => vec![],
                            }
                        }
                        "Assume" => {
                            scenario_description.assumptions = match directive.value() {
                                Some(s) => UnitName::from_list(s, "test")?,
                                None => vec![],
                            }
                        }
                        &_ => (),
                    }
                }
                &_ => (),
            }
        }
        Ok(scenario_description)
    }

    pub fn id(&self) -> &UnitName {
        &self.id
    }

    /// Returns true if this scenario is supported on the named jig.
    pub fn supports_jig(&self, name: &UnitName) -> bool {
        self.jigs.contains(name)
    }

    /// Determine if a unit is compatible with this system.
    pub fn is_compatible(&self,
                         manager: &UnitManager,
                         _: &Config)
                         -> Result<Vec<UnitName>, UnitIncompatibleReason> {
        // If there is at least one jig present, ensure that it is loaded.
        if self.jigs.len() > 0 {
            let mut loaded = false;
            for jig_name in &self.jigs {
                if manager.jig_is_loaded(&jig_name) {
                    loaded = true;
                }
            }
            if !loaded {
                return Err(UnitIncompatibleReason::IncompatibleJig);
            }
        }

        // Build the dependency graph, but don't use the result.
        // This is because right now, we're just concerned with
        // whether the dependencies are satisfied.
        self.get_test_order(manager)
    }

    pub fn select(&self,
                  manager: &UnitManager,
                  config: &Config)
                  -> Result<Scenario, UnitIncompatibleReason> {
        let test_order = self.is_compatible(manager, config)?;
        Ok(Scenario::new(self, test_order, manager))
    }

    pub fn get_test_order(&self,
                          manager: &UnitManager)
                          -> Result<Vec<UnitName>, UnitIncompatibleReason> {

        // Create a new dependency graph
        let mut graph = Dependy::new();

        // Add each possible test into the dependency graph
        {
            let tests_rc = manager.get_tests();
            let tests = tests_rc.borrow();
            for (test_name, test) in tests.iter() {
                if self.assumptions.contains(test_name) {
                    let assumption_dep = AssumptionDependency::new(test_name.to_string());
                    graph.add_dependency(&assumption_dep);
                } else {
                    graph.add_dependency(&*test.lock().unwrap());
                }
            }
        }

        let mut test_names = vec![];
        for test_name in &self.tests {
            test_names.push(test_name.to_string());
        }

        let test_sequence_strings = graph.resolve_named_dependencies(&test_names)?;
        let mut test_order = vec![];
        for test_name_string in test_sequence_strings {
            // Unwrap, because the name ought to be valid due to it being internally generated.
            let test_name = UnitName::from_str(test_name_string.as_str(), "test")
                .expect("Invalid test name generated");

            // Only add the test to the test order if it's not an assumption.
            if !self.assumptions.contains(&test_name) {
                test_order.push(test_name);
            }
        }

        // let test_order = trimmed_order;
        Ok(test_order)
    }
}

pub struct Scenario {
    name: UnitName,
    test_sequence: Vec<Arc<Mutex<Test>>>,
    tests: HashMap<UnitName, Arc<Mutex<Test>>>,
}

impl Scenario {
    pub fn new(desc: &ScenarioDescription,
               test_order: Vec<UnitName>,
               manager: &UnitManager)
               -> Scenario {

        let mut tests = HashMap::new();
        let mut test_sequence = vec![];

        for test_name in test_order {
            let test = manager.get_test(&test_name).expect("Unable to check out requested test from library");
            test_sequence.push(test.clone());
            tests.insert(test_name, test);
        }

        Scenario {
            name: desc.id.clone(),
            tests: tests,
            test_sequence: test_sequence,
        }
    }

    pub fn activate(&self) -> Result<(), UnitActivateError> {
        Ok(())
    }

    pub fn deactivate(&self) -> Result<(), UnitDeactivateError> {
        Ok(())
    }

    pub fn uses_test(&self, test_name: &UnitName) -> bool {
        self.tests.get(test_name).is_some()
    }
}

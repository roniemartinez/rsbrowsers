use pyo3::Bound;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rsbrowsers::{Browser, BrowserFinder};
use std::convert::Infallible;

struct PyBrowser(Browser);

impl<'py> IntoPyObject<'py> for PyBrowser {
    type Target = PyDict;
    type Output = Bound<'py, PyDict>;
    type Error = Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let dict = PyDict::new(py);

        dict.set_item("display_name", self.0.display_name).expect("Cannot set display_name.");
        dict.set_item("path", self.0.path).expect("Cannot set path.");
        dict.set_item("browser_type", self.0.browser_type).expect("Cannot set browser_type.");
        dict.set_item("version", self.0.version).expect("Cannot set version.");

        Ok(dict)
    }
}

#[pyfunction(name = "browsers")]
fn all() -> PyResult<Vec<PyBrowser>> {
    let browsers = BrowserFinder::new().all().map(|browser| PyBrowser(browser)).collect();
    Ok(browsers)
}

#[pyfunction(signature = (browser, version="*"))]
fn get(browser: String, version: &str) -> PyResult<Option<PyBrowser>> {
    match BrowserFinder::new().with_type(browser).with_version(version.to_string()).all().next() {
        Some(browser) => Ok(Some(PyBrowser(browser))),
        None => Ok(None),
    }
}

#[pyfunction(signature = (browser, version=None, url=None, args=None))]
fn launch(browser: String, version: Option<String>, url: Option<String>, args: Option<Vec<String>>) {
    let mut finder = BrowserFinder::new().with_type(browser);
    if let Some(v) = version {
        finder = finder.with_version(v);
    }
    let args = args.unwrap_or_else(|| vec![]);
    finder.launch(args.as_slice());
}

#[pymodule]
fn browsers(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(all, m)?)?;
    m.add_function(wrap_pyfunction!(get, m)?)?;
    m.add_function(wrap_pyfunction!(launch, m)?)?;
    Ok(())
}

import subprocess
from typing import Iterator, Optional, Sequence

from typing import TypedDict


class Browser(TypedDict):
    browser_type: str
    path: str
    display_name: str
    version: str


def browsers() -> Iterator[Browser]:
    """
    Iterates over installed browsers.

    :return: Iterator of Tuple of browser key and browser information.
    """
    ...


def get(browser: str, version: str = "*") -> Optional[Browser]:
    """
    Returns the information for the provided browser key.

    :param browser: Any of "chrome", "chrome-canary", "firefox", "firefox-developer", "firefox-nightly", "opera", ...
                    see LINUX_DESKTOP_ENTRY_LIST, OSX_BROWSER_BUNDLE_LIST and WINDOWS_REGISTRY_BROWSER_NAMES for values
    :param version: Version string (supports wildcard, e.g. 100.*)
    :return: Dictionary containing "path", "display_name" and "version".
    """
    ...


def launch(
        browser: str, version: str = "*", url: Optional[str] = None, args: Optional[Sequence[str]] = None
) -> Optional[subprocess.Popen]:
    """
    Launches a web browser.

    :param browser: Browser key.
    :param version: Version string (supports wildcard, e.g. 100.*)
    :param url: URL.
    :param args: Arguments to be passed to the browser.
    """
    ...

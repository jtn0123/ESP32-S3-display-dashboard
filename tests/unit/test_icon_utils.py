import sys
from xml.etree import ElementTree as ET

import os

# Ensure src directory is on path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', 'src')))

from utils.icon_utils import insert_title


def test_insert_title_no_attributes():
    svg = "<svg></svg>"
    result = insert_title(svg, "Test Icon")
    root = ET.fromstring(result)
    assert root.tag == "svg"
    assert root.find("title").text == "Test Icon"


def test_insert_title_preserves_attributes():
    svg = '<svg viewBox="0 0 24 24" width="24"></svg>'
    result = insert_title(svg, "Attr Icon")
    root = ET.fromstring(result)
    assert root.attrib.get("viewBox") == "0 0 24 24"
    assert root.attrib.get("width") == "24"
    assert root.find("title").text == "Attr Icon"


def test_insert_title_replaces_existing():
    svg = '<svg viewBox="0 0 24 24"><title>Old</title><path d="" /></svg>'
    result = insert_title(svg, "New Title")
    root = ET.fromstring(result)
    titles = root.findall("title")
    assert len(titles) == 1
    assert titles[0].text == "New Title"
    assert root.attrib.get("viewBox") == "0 0 24 24"
    # Ensure other elements remain
    assert root.find("path") is not None

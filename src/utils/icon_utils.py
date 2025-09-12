"""Utility functions for manipulating SVG icons."""
from xml.etree import ElementTree as ET


def insert_title(svg_content: str, title: str) -> str:
    """Insert or replace the <title> element in an SVG string.

    The function preserves existing attributes and elements while ensuring the
    SVG contains a single <title> element with the provided text.

    Args:
        svg_content: The raw SVG XML as a string.
        title: Title text to insert inside the SVG.

    Returns:
        The modified SVG string containing the <title> element.
    """
    # Parse the SVG content
    try:
        root = ET.fromstring(svg_content)
    except ET.ParseError:
        # If parsing fails, return original content unmodified
        return svg_content

    # Find existing <title> element
    title_elem = root.find("{http://www.w3.org/2000/svg}title")
    if title_elem is None:
        title_elem = root.find("title")

    if title_elem is not None:
        title_elem.text = title
    else:
        # Insert new <title> as first child
        new_title = ET.Element("title")
        new_title.text = title
        root.insert(0, new_title)

    # Serialize back to string
    return ET.tostring(root, encoding="unicode")

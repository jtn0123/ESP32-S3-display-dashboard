"""Utility functions for image manipulation."""
from __future__ import annotations


def change_orientation(image, orientation):
    """Rotate an image based on the provided orientation.

    Parameters
    ----------
    image : object
        An object implementing a ``rotate(angle)`` method.
    orientation : str or int
        Desired orientation. Supported values include the strings
        ``"portrait"``, ``"landscape"``, ``"reverse_portrait"`` and
        ``"reverse_landscape"``. Alternatively, an integer angle of
        ``0``, ``90``, ``180`` or ``270`` degrees may be supplied.

    Returns
    -------
    object
        The rotated image instance.

    Raises
    ------
    ValueError
        If an unknown orientation is provided.
    """
    mapping = {
        "portrait": 0,
        "landscape": 90,
        "reverse_portrait": 180,
        "reverse_landscape": 270,
    }

    angle = None
    if isinstance(orientation, int):
        if orientation in {0, 90, 180, 270}:
            angle = orientation
    else:
        angle = mapping.get(orientation)

    if angle is None:
        raise ValueError(f"Unknown orientation: {orientation}")

    return image.rotate(angle)

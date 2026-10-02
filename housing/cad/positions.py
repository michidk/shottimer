"""Positions are named while looking at the display."""
USB_ANGLES = {'right': 0, 'up': 90, 'left': 180}

def validate_positions(usb, magnet):
    if usb not in USB_ANGLES:
        raise ValueError('USB position must be left, right, or up')
    if magnet not in ('left', 'right'):
        raise ValueError('Magnet position must be left or right')
    if usb == magnet:
        raise ValueError('USB and magnet cannot occupy the same side')

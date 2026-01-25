# QMK Layout Helper <img src="resources/icon.svg" align="right" width="15%"/>

QMK Layout Helper provides a live on‑screen overlay of your keyboard, mirroring the active base and momentary layers. It is especially helpful when learning complex multi‑layer layouts or using boards with missing legends. The overlay updates instantly on layer changes, querying the current keymap and layer state so the view always matches the firmware.

It reflects the actual active layer stack, i.e. base and momentary layers, so the shown keys always correspond to the current effective layout.

**Supported protocols:**
- **VIA** – Requires a keyboard info JSON file (exported from QMK)
- **VIAL** – Automatically fetches layout definition from firmware (no JSON required)

<img src=".github/assets/demo.gif" alt="QMK Layout Helper in action">

## Setup

Stock QMK/VIA/VIAL firmware does not expose layer change events to the host, so a minimal firmware change is required to send layer state updates via RAW HID. The same firmware modifications work for both VIA and VIAL keyboards.

### Firmware Modifications

#### For VIA keyboards

Add the following to your `rules.mk` file to enable VIA and RAW HID support:
```
VIA_ENABLE = yes
RAW_ENABLE = yes
```

#### For VIAL keyboards

Add the following to your `rules.mk` file to enable VIAL and RAW HID support:
```
VIAL_ENABLE = yes
RAW_ENABLE = yes
```

### Layer State Reporting (Required)

Add the following to your `keymap.c` file to enable active layer reporting:
  ```c
#include "raw_hid.h"

// Notify about layer changes
layer_state_t layer_state_set_user(layer_state_t state) {
    uint8_t data[RAW_EPSIZE] = {0};
    data[0] = 0xFF;
    data[1] = sizeof(layer_state_t);
    memcpy(&data[2], &default_layer_state, sizeof(layer_state_t));
    memcpy(&data[2 + sizeof(layer_state_t)], &state, sizeof(layer_state_t));
    raw_hid_send(data, RAW_EPSIZE);
    return state;
}
```

### Key Press Highlighting (Optional)

If you want the currently pressed keys to be highlighted, also add the following to your `keymap.c`:
```c
// Notify about key press/release events
bool process_record_user(uint16_t keycode, keyrecord_t *record) {
    static uint8_t data[RAW_EPSIZE];
    data[0] = 0xF1;
    data[1] = record->event.key.row;
    data[2] = record->event.key.col;
    data[3] = record->event.pressed ? 1 : 0;
    raw_hid_send(data, RAW_EPSIZE);
    return true;
}
```

### Building the Firmware

Compile and flash the modified firmware to your keyboard:
```sh
qmk compile -kb <your_keyboard> -km <your_keymap>
```

### Additional Setup for VIA

For VIA keyboards, you also need to obtain the keyboard information JSON file:
```sh
qmk info -kb <your_keyboard> -m -f json > keyboard_info.json
```
This is the input file for the QMK Layout Helper containing the keyboard layout information required for rendering the overlay.

**Note:** VIAL keyboards do not require this step – the layout definition is fetched directly from the firmware.

## Usage

### VIAL Keyboards

1. Click "Scan for devices" to detect connected VIAL keyboards
2. Select your keyboard from the list
3. Choose the desired layout variant (if available)

### VIA Keyboards

1. Select "VIA" as the protocol type
2. Load the keyboard information JSON file obtained during setup
3. Select the correct layout for your keyboard

<img src=".github/assets/settings_window.png" alt="Settings window screenshot" width="60%">

When "Remember settings" is checked, the selected options will be saved to a settings.ini file. To modify settings later, either edit the settings.ini file manually or delete it to trigger the settings window on the next launch.

## Troubleshooting

### VIAL device not detected
- Ensure your keyboard has VIAL firmware installed (not just VIA)
- Check that the keyboard is properly connected and recognized by your OS
- On Linux, you may need to add udev rules for HID access

### Layer changes not reflected
- Verify that the `layer_state_set_user` function is included in your firmware
- Ensure `RAW_ENABLE = yes` is in your `rules.mk`
- Rebuild and reflash your firmware after making changes

### Key highlighting not working
- Verify that the `process_record_user` function is included in your firmware
- Note: If you have other code in `process_record_user`, make sure to integrate the key reporting code appropriately

## License & Attribution

Parts of this project are based on code from [the VIA project](https://github.com/the-via/app), which is licensed under the GNU General Public License v3.0.
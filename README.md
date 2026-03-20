# KeyPeek <img src="resources/icon.svg" align="right" width="15%"/>

KeyPeek provides a live on-screen overlay of your keyboard, mirroring the active base and momentary layers. It is especially useful when learning complex multi-layer layouts or using boards with missing legends. The overlay updates instantly when layers change, so the view always matches your firmware state. KeyPeek currently supports QMK, Vial, and ZMK keyboards.

<img src=".github/assets/demo.gif" alt="KeyPeek in action">

## Setup

KeyPeek requires a small firmware module because stock QMK/Vial/ZMK firmware does not expose live layer-change events.
The module adds that event stream over the device connection, so the overlay stays in sync with your active layers in real time.

### QMK and Vial

1. In your QMK userspace (or `qmk_firmware`) root, add the module repo:
   
   ```sh
   mkdir -p modules
   git submodule add https://github.com/srwi/qmk-modules.git modules/srwi
   git submodule update --init --recursive
   ```
   
2. In your keymap folder, add `srwi/keypeek_layer_notify` to `keymap.json`:
   
   ```json
   {
     "modules": [
       "srwi/keypeek_layer_notify"
     ]
   }
   ```
   
3. In the same keymap folder, enable RAW HID and VIA in `rules.mk`:
   
   ```make
   RAW_ENABLE = yes
   VIA_ENABLE = yes
   ```
   
4. Build and flash your firmware:
   
   ```sh
   qmk compile -kb <your_keyboard> -km <your_keymap>
   ```
   
5. **QMK only:** Export layout information to `keyboard_info.json`:
   
   ```sh
   qmk info -kb <your_keyboard> -m -f json > keyboard_info.json
   ```
   
   This last step is only required for QMK keyboards, because VIA does not provide physical layout data directly over the connection. Vial keyboards do not require this step, as the layout data is transmitted when connecting the keyboard to KeyPeek.

### ZMK

1. Add the KeyPeek module to your `zmk-config/config/west.yml`:

  ```yaml
  manifest:
    remotes:
      - name: zmkfirmware
        url-base: https://github.com/zmkfirmware
      - name: zzeneg # <-- required for Raw HID module
        url-base: https://github.com/zzeneg
      - name: srwi # <-- required for KeyPeek module
        url-base: https://github.com/srwi
    projects:
      - name: zmk
        remote: zmkfirmware
        revision: main
        import: app/west.yml
      - name: zmk-raw-hid # <-- Raw HID module
        remote: zzeneg
        revision: main
      - name: zmk-keypeek-layer-notifier # <-- KeyPeek module
        remote: srwi
        revision: master
  ```

2. Add the `raw_hid_adapter` as an additional shield to your build, e.g. in `build.yaml`:

  ```yaml
  include:
    - board: nice_nano_v2
      shield: <existing shields> raw_hid_adapter # <-- required for Raw HID support
      snippet: studio-rpc-usb-uart # <-- required for ZMK Studio support
  ```

3. Enable ZMK Studio support in your `.conf` file:

  ```conf
  CONFIG_ZMK_STUDIO=y
  ```

If your keyboard does not support ZMK Studio yet, adding support is described in the [ZMK documentation](https://zmk.dev/docs/features/studio#adding-zmk-studio-support-to-a-keyboard).

KeyPeek will then read layout and keymap directly from the device for ZMK without requiring additional configuration.

> [!NOTE]
> If the keyboard has been paired via Bluetooth before enabling raw HID support, re-pairing may be necessary to allow the new communication channel.

## Usage

Devices are scanned when the app starts. For QMK you will be prompted to select the `keyboard_info.json` generated from your keymap when you connect. For Vial and ZMK, just select the connected device from the dropdown, since they provide layout information directly.

Appearance settings are saved to `settings.ini` in the app directory.

<img src=".github/assets/settings_window.png" alt="Settings window screenshot" width="60%">

# License & Attribution

Parts of this project are based on code from [the VIA project](https://github.com/the-via/app), which is licensed under the GNU General Public License v3.0.

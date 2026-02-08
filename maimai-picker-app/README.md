# maimai picker

Flutter app for randomly selecting maimai DX songs by internal level range. Designed for glove-friendly interaction with physical button controls.

## Features

- 🎮 **Hardware Button Controls**
  - **Android**: Volume buttons (Up/Down to adjust range, both simultaneously to trigger random)
  - **iOS/macOS**: Arrow keys (Up/Down to adjust range, Space/Enter to trigger random)
- 🎯 **Internal Level Filtering**: Select songs within precise internal level ranges (e.g., 12.5-12.6)
- 🎨 **Glove-Friendly UI**: Large touch targets (80x80+) for arcade use
- 📱 **Cross-Platform**: iOS, macOS, Android support
- ⚙️ **Configurable Backend**: Connect to your own maimai-bot backend instance

## Architecture

- **State Management**: BLoC + Cubit pattern
- **Feature-First Structure**: Organized by features (song_selection, settings)
- **Material 3**: Modern Material Design with dark theme optimized for arcades

## Prerequisites

- Flutter SDK (latest stable)
- Running song-info-server instance (default: `http://localhost:3001`)
- (Optional) Running record-collector-server instance for personal scores (default: `http://localhost:3000`)

## Setup

1. Install dependencies:
```bash
cd maimai-picker-app
flutter pub get
```

2. Configure backend URLs:
   - Launch the app
   - Tap the settings icon (top right)
   - Enter your Song Info Server URL (required for basic functionality)
   - (Optional) Enter your Record Collector Server URL for personal scores
   - Tap Save

   **Note**: The app works in degraded mode with only Song Info Server configured. Personal achievement data will not be displayed without Record Collector Server.

## Running

```bash
flutter run
```

Or for specific platforms:
```bash
flutter run -d ios
flutter run -d macos
flutter run -d android
```

## How to Use

### Main Screen

1. **Adjust Level Range**:
   - Tap **-** / **+** buttons to adjust the range start
   - Or use volume buttons (Android) / arrow keys (macOS)

2. **Adjust Gap**:
   - Tap gap buttons (0.05, 0.1, 0.2, 0.5) to change range width

3. **Get Random Song**:
   - Tap **RANDOM** button
   - Or press both volume buttons simultaneously (Android)
   - Or press Space/Enter (macOS/iOS)

### Song Display

When a song is selected, you'll see:
- Jacket image (300x300)
- Song title
- Chart information (STD/DX, difficulty, level)
- Internal level (⚡ 13.7)
- Your achievement (if played)
- FC/Sync status (if applicable)

### Settings Screen

- Configure Song Info Server URL (required)
- Configure Record Collector Server URL (optional)
- Reset to default settings

## Hardware Input Reference

| Action | Android | iOS/macOS |
|--------|---------|-----------|
| Increment range | Volume Up | Arrow Up |
| Decrement range | Volume Down | Arrow Down |
| Random song | Both volumes together | Space / Enter |

## Project Structure

```
lib/
├── core/
│   ├── constants/        # App constants (URLs, level bounds)
│   ├── theme/            # Material 3 theme
│   └── widgets/          # Shared widgets
├── features/
│   ├── song_selection/
│   │   ├── bloc/         # State management (Cubits)
│   │   ├── data/         # Models, repositories
│   │   └── presentation/ # UI screens and widgets
│   └── settings/
│       ├── bloc/         # Settings state management
│       └── presentation/ # Settings screen
└── main.dart
```

## Dependencies

- `flutter_bloc` - State management
- `equatable` - Value equality
- `dio` - HTTP client
- `cached_network_image` - Image caching
- `volume_listener` - Volume button handling (Android)
- `shared_preferences` - Settings persistence

## Backend Integration

The app connects to two separate backend servers:

### Song Info Server (Required)

**Endpoint**: `GET /api/songs/random`

**Query Parameters**:
- `min_level` (float): Minimum internal level
- `max_level` (float): Maximum internal level

**Response**: Song data with jacket image URL, chart info, and internal level

**Cover Images**: `GET /api/cover/{image_name}`

Returns jacket image files for display.

### Record Collector Server (Optional)

Used to fetch personal achievement data for selected songs. If not configured, the app will display songs without personal scores.

**Note**: The app works with only Song Info Server configured. Personal achievement, FC status, and Sync status will not be displayed without Record Collector Server.

## Building for Release

### Android
```bash
flutter build apk --release
flutter build appbundle --release
```

### iOS
```bash
flutter build ipa --release
```

### macOS
```bash
flutter build macos --release
```

### CI Build (GitHub Actions)

The workflow at `.github/workflows/build-apk.yml` builds a signed release APK on push to `main` (when `maimai-picker-app/` changes) or manually via `workflow_dispatch`.

**Required GitHub Secrets:**

| Secret | Value |
|--------|-------|
| `KEYSTORE_BASE64` | Base64-encoded release keystore (`base64 -i release-keystore.jks`) |
| `KEY_ALIAS` | Key alias in the keystore |
| `KEY_PASSWORD` | Key password |
| `STORE_PASSWORD` | Keystore password |

**Generate a keystore (if you don't have one):**

```bash
keytool -genkey -v -keystore release-keystore.jks -keyalg RSA -keysize 2048 -validity 10000 -alias maimai-picker
```

**Encode it for the secret:**

```bash
base64 -i release-keystore.jks | pbcopy
```

Paste the clipboard contents into the `KEYSTORE_BASE64` secret in GitHub → Settings → Secrets and variables → Actions.

## Troubleshooting

### Backend Connection Issues
- Ensure Song Info Server is running on the configured URL (required)
- Ensure Record Collector Server is running if you want personal scores (optional)
- Check network connectivity
- Verify backend URLs in Settings (include `http://` prefix)

### Volume Buttons Not Working (Android)
- Ensure app has necessary permissions
- Volume buttons only work when app is in foreground
- System volume UI should be hidden automatically

### No Songs Found
- Check Song Info Server connection (required for song data)
- Try widening the level range (increase gap)
- Verify that song data is loaded in Song Info Server

### Personal Scores Not Showing
- Ensure Record Collector Server is running and configured in Settings
- Verify you have played songs in the selected level range
- Check Record Collector Server connection

## License

Same license as maimai-bot parent project.

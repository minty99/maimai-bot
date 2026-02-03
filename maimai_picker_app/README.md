# Maimai DX Song Randomizer

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
- Running maimai-bot backend instance (default: `http://localhost:3000`)

## Setup

1. Install dependencies:
```bash
cd flutter_maimai_randomizer
flutter pub get
```

2. Configure backend URL:
   - Launch the app
   - Tap the settings icon (top right)
   - Enter your backend URL
   - Tap Save

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

- Configure backend URL
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

The app connects to the maimai-bot backend's REST API:

**Endpoint**: `GET /api/songs/random`

**Query Parameters**:
- `min_level` (float): Minimum internal level
- `max_level` (float): Maximum internal level

**Response**: Song data with jacket image URL, chart info, internal level, and player scores

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

## Troubleshooting

### Backend Connection Issues
- Ensure backend is running on the configured URL
- Check network connectivity
- Verify backend URL in Settings (include `http://` prefix)

### Volume Buttons Not Working (Android)
- Ensure app has necessary permissions
- Volume buttons only work when app is in foreground
- System volume UI should be hidden automatically

### No Songs Found
- Ensure you have played songs in the selected level range
- Try widening the level range (increase gap)
- Check backend connection

## License

Same license as maimai-bot parent project.

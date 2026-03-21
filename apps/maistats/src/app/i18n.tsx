import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type PropsWithChildren,
} from 'react';

export type SupportedLanguage = 'ko' | 'en';
export type LanguagePreference = 'system' | SupportedLanguage;

export const LANGUAGE_STORAGE_KEY = 'maistats.language';

const LOCALE_BY_LANGUAGE: Record<SupportedLanguage, string> = {
  ko: 'ko-KR',
  en: 'en-US',
};

function defineTranslations<T extends Record<string, string>>(value: {
  ko: T;
  en: { [K in keyof T]: string };
}) {
  return value;
}

const translations = defineTranslations({
  ko: {
    'nav.home': 'Home',
    'nav.setup': 'Setup',
    'nav.scores': 'Scores',
    'nav.rating': 'Rating',
    'nav.playlogs': 'Playlogs',
    'nav.picker': 'Picker',
    'nav.settings': 'Settings',
    'nav.primary': 'Primary',
    'nav.openPages': '페이지 목록 열기',
    'common.filters': 'Filters',
    'common.chartTraits': '채보 특성',
    'common.recordTraits': '기록 특성',
    'common.close': '닫기',
    'common.connect': '연결',
    'common.connecting': '연결 중...',
    'common.all': 'ALL',
    'common.apply': '적용',
    'common.search': '검색',
    'common.loadingCharts': '차트 불러오는 중...',
    'common.loadingPlaylogs': '플레이 기록 불러오는 중...',
    'common.loadingVersions': '버전 불러오는 중...',
    'common.jacket': 'Jacket',
    'common.compact': 'Compact',
    'common.title': 'Title',
    'common.chart': 'Chart',
    'common.levelShort': 'Lv',
    'common.achievementShort': 'Achv',
    'common.rating': 'Rating',
    'common.rank': 'Rank',
    'common.fc': 'FC',
    'common.sync': 'Sync',
    'common.dx': 'DX',
    'common.lastPlayed': 'Last Played',
    'common.playCount': 'Play Count',
    'common.version': 'Version',
    'common.track': 'Track',
    'common.type': 'Type',
    'common.diff': 'Diff',
    'common.achievement': 'Achievement',
    'common.error': '에러',
    'common.min': 'MIN',
    'common.max': 'MAX',
    'common.from': 'FROM',
    'common.to': 'TO',
    'common.none': '없음',
    'common.ready': 'Ready',
    'units.songs': '{{count}}곡',
    'units.credits': '{{count}} 크레딧',
    'units.daysAgo': '{{count}}일 전',
    'player.connected': 'Connected player',
    'player.totalPlayCount': 'Total play count',
    'home.connect.title': 'Record Collector 연결',
    'home.connect.description': 'Record Collector 서버 URL을 입력하고 연결을 확인합니다.',
    'home.connect.serverUrl': '서버 URL',
    'home.connect.placeholder': 'https://your-server.example.com',
    'home.connect.failed': '연결 실패: {{message}}',
    'home.connect.success': '연결 성공! 플레이어: {{name}}',
    'home.connect.goToScores': 'Scores로 이동 →',
    'home.intro.description': 'maistats는 개인 수집 서버와 연결해 Scores, Playlogs, Rating, Picker를 한 화면에서 확인하는 기록 관리 앱입니다.',
    'home.intro.helper': 'Record Collector를 직접 운영하는 환경을 전제로 하며, 처음이라면 설정 가이드부터 확인하면 됩니다.',
    'home.startCard.title': '시작 방법',
    'home.startCard.body': 'collector를 아직 준비하지 않았다면 설정 가이드에서 서버 실행과 연결 순서를 확인하세요.',
    'home.discordCard.title': 'Discord Bot',
    'home.discordCard.body': 'Discord Bot에서도 스코어와 최근 플레이를 바로 조회할 수 있습니다.',
    'home.openSetup': 'Record Collector 설정 가이드',
    'home.guide.title': '서버 실행 가이드',
    'home.guide.description': 'Record Collector 서버가 없다면 아래 안내를 따라 직접 실행하세요.',
    'home.guide.step1Title': 'compose.yaml 파일 생성',
    'home.guide.step1BodyA': '서버를 실행할 폴더에 아래 내용으로 ',
    'home.guide.step1BodyB': ' 파일을 만듭니다. ',
    'home.guide.step1BodyC': '와 ',
    'home.guide.step1BodyD': '에 maimaidx-eng.com 계정 정보를 입력하세요.',
    'home.guide.step2Title': 'Docker Compose 실행',
    'home.guide.step2BodyA': 'Docker가 설치된 환경에서 ',
    'home.guide.step2BodyB': '이 있는 폴더에서 아래 명령어로 컨테이너를 시작합니다.',
    'home.guide.step2BodyC': '첫 실행 시 이미지 다운로드 후 maimaidx-eng.com 로그인이 진행됩니다. 서버가 준비되면 ',
    'home.guide.step2BodyD': ' 엔드포인트가 200을 반환합니다.',
    'home.guide.step3Title': '외부 접근 설정 (선택)',
    'home.guide.step3Body': '외부에서 접근하려면 서버를 공개 IP 또는 도메인으로 노출하고 해당 주소를 입력하세요. ngrok, Cloudflare Tunnel 등을 활용할 수 있습니다.',
    'home.guide.step4Title': 'URL 연결',
    'home.guide.step4BodyA': '서버가 준비되면 위 입력창에 서버 URL을 입력하고 ',
    'home.guide.step4BodyB': ' 버튼을 클릭하세요. 연결에 성공하면 자동으로 Scores 페이지로 이동합니다.',
    'home.discord.title': 'Discord Bot',
    'home.discord.description': 'Discord 서버에 maistats 봇을 추가하면 ',
    'home.discord.descriptionTail': ' 명령어로 스코어와 최근 플레이 기록을 바로 조회할 수 있습니다.',
    'home.discord.addButton': 'Discord Bot 추가하기',
    'home.footer.aliases': '곡 제목의 alias는 ',
    'home.footer.aliasesTail': '으로부터 허가를 받아 가져왔습니다.',
    'home.footer.parsing': '곡의 파싱은 ',
    'home.footer.parsingTail': '를 참고했습니다.',
    'home.footer.source': 'maistats의 소스 코드는 ',
    'home.footer.sourceTail': '에 공개되어 있습니다.',
    'home.footer.developer': '개발자:',
    'home.footer.copyrightA': '본 사이트는 개인 성과 기록 및 추적을 위해 만든 ',
    'home.footer.copyrightB': '의 팬 사이트이며, 사이트 내에 사용된 게임 관련 컨텐츠의 저작권은 ',
    'home.footer.copyrightC': ' 및 ',
    'home.footer.copyrightOwners': '각 소유자들',
    'home.footer.copyrightD': '에게 있습니다.',
    'settings.title': 'Connections',
    'settings.description': 'Song Info와 Record Collector 연결 정보를 관리합니다.',
    'settings.songInfoWarning': '⚠ 디버깅 목적이 아니라면 변경하지 마세요.',
    'settings.language.title': 'Language',
    'settings.language.description': '앱 언어를 기기 언어 또는 직접 선택한 언어로 설정합니다.',
    'settings.language.label': 'App language',
    'settings.language.helperSystem': '현재 기기 언어를 따라갑니다. 현재 적용 언어: {{language}}',
    'settings.language.helperManual': '현재 적용 언어: {{language}}',
    'settings.language.optionSystem': 'System default',
    'settings.language.optionKo': '한국어',
    'settings.language.optionEn': 'English',
    'settings.theme.title': 'Theme',
    'settings.theme.description': '앱의 색상 테마를 선택합니다.',
    'settings.theme.label': 'Color theme',
    'settings.theme.optionSystem': 'System default',
    'settings.theme.optionLight': 'Light',
    'settings.theme.optionDark': 'Dark',
    'settings.recordCollector.success': '연결 성공! 플레이어: {{name}}',
    'settings.recordCollector.failed': '연결 실패: {{message}}',
    'scores.resetAll': '전체 초기화',
    'scores.searchLabel': '검색 (곡명/alias/버전/레벨)',
    'scores.searchPlaceholder': '예: VERTeX, 버텍스, PRiSM, 14+',
    'scores.chartType': '채보 유형',
    'scores.difficulty': '난이도',
    'scores.level': '레벨',
    'scores.levelMin': '레벨 최소',
    'scores.levelMax': '레벨 최대',
    'scores.score': '스코어',
    'scores.achievementMin': '달성률 최소',
    'scores.achievementMax': '달성률 최대',
    'scores.version': '버전',
    'scores.daysSince': '경과일',
    'scores.daysMin': '경과일 최소',
    'scores.daysMax': '경과일 최대',
    'scores.chartsTitle': 'Charts',
    'scores.chartsDescription': '점수 데이터와 차트 메타데이터를 함께 확인합니다. 회색 소수점은 추정 내부레벨입니다.',
    'scores.layout': 'Charts layout',
    'scores.versionAll': 'ALL',
    'scores.versionNew': 'NEW',
    'scores.versionOld': 'OLD',
    'playlogs.searchLabel': '검색 (곡명/alias/시각)',
    'playlogs.searchPlaceholder': '예: 2026/02/25, BUDDiES, 배드애플',
    'playlogs.showAll': '전체 플레이 기록 보기',
    'playlogs.dayLabel': '플레이 날짜 (maimai day 04:00 기준)',
    'playlogs.summaryAll': '전체: {{songCount}}곡',
    'playlogs.summaryDay': '{{songCount}}곡 · {{creditCount}} 크레딧',
    'playlogs.bestOnly': '곡/채보별 최고 기록만 보기',
    'playlogs.newRecordOnly': 'new record만 보기',
    'playlogs.layout': 'Playlogs layout',
    'playlogs.creditNumber': 'Credit #',
    'playlogs.playedAt': 'Played At',
    'playlogs.dayOption': '{{date}} ({{credits}} credits)',
    'rating.title': 'RATING',
    'rating.description': 'NEW 상위 15곡과 OLD 상위 35곡의 레이팅 합계입니다. 보면상수가 알려지지 않은 곡의 경우 계산값이 잘못될 수 있습니다.',
    'rating.current': 'Current Rating',
    'rating.newTop15': 'NEW TOP 15',
    'rating.oldTop35': 'OLD TOP 35',
    'rating.avg': 'avg {{value}}',
    'rating.avgProjection': 'avg {{avg}}, ~{{projection}}',
    'rating.newDescription': 'NEW 분류 상위 15곡. 카드를 클릭하면 Song Detail을 엽니다.',
    'rating.oldDescription': 'OLD 분류 상위 35곡. 카드를 클릭하면 Song Detail을 엽니다.',
    'rating.openSongDetail': '{{title}} Song Detail 열기',
    'picker.filterSettings': 'Filter Settings',
    'picker.selectAll': '전체 선택',
    'picker.clearAll': '전체 해제',
    'picker.chooseVersions': 'Choose versions',
    'picker.lastPlayedDays': 'Last played (days)',
    'picker.songInfoRequired': 'Song Info URL이 비어 있습니다.',
    'picker.noSongs': '조건에 맞는 곡이 없습니다.',
    'picker.noSongsHelp': '조건에 맞는 곡이 없습니다. 범위를 넓히거나 필터를 완화해보세요.',
    'picker.picking': '선곡 중...',
    'picker.pickFailed': '랜덤 선곡 실패',
    'picker.noData': 'No Data',
    'picker.unplayed': 'UNPLAYED',
    'picker.fcEmpty': 'FC -',
    'picker.syncEmpty': 'SYNC -',
    'picker.metaWithData': 'Last played: {{lastPlayed}}  •  Play count: {{playCount}}',
    'picker.metaNoData': 'Last played: No Data  •  Play count: No Data',
    'picker.pickedFrom': '{{count}}곡 중 선곡',
    'picker.pickedFromNoData': '집계 가능한 곡 수 없음',
    'picker.versionsSelected': '{{count}} versions selected',
    'picker.controlTitle': 'Picker Controls',
    'picker.filters': 'Filters',
    'picker.random': 'RANDOM',
    'picker.selection': 'Selection',
    'picker.statusPicking': 'Picking',
    'picker.statusError': 'Error',
    'picker.statusEmpty': 'Empty',
    'picker.statusPicked': 'Picked',
    'picker.statusReady': 'Ready',
    'songDetail.title': 'Song Detail',
    'songDetail.refreshing': '갱신 중...',
    'songDetail.refresh': 'Score 갱신',
    'songDetail.refreshUnavailable': '곡 식별 정보가 부족해서 새로고침할 수 없습니다.',
    'songDetail.empty': '조회 가능한 상세 데이터가 없습니다.',
    'history.title': 'History',
    'history.description': 'playlogs 기준으로 최고 달성률이 갱신된 시점만 표시합니다.',
    'history.loading': 'playlogs를 불러오는 중입니다.',
    'history.empty': '이 채보에 대한 최고기록 변동 이력을 playlogs에서 찾지 못했습니다.',
    'history.graphLabel': '{{title}} 최고 달성률 변화 그래프',
    'history.axisAchievement': 'Achievement',
    'history.axisTime': 'Time',
    'app.missingUrls': 'Scores와 Playlogs 페이지는 Song Info와 Record Collector URL이 모두 필요합니다.',
    'api.enterUrl': 'URL을 입력하세요.',
    'api.connectionFailed': 'HTTP {{status}} 응답을 받았습니다.',
    'api.recordCollectorRequired': 'Record Collector URL이 비어 있습니다.',
  },
  en: {
    'nav.home': 'Home',
    'nav.setup': 'Setup',
    'nav.scores': 'Scores',
    'nav.rating': 'Rating',
    'nav.playlogs': 'Playlogs',
    'nav.picker': 'Picker',
    'nav.settings': 'Settings',
    'nav.primary': 'Primary',
    'nav.openPages': 'Open page list',
    'common.filters': 'Filters',
    'common.chartTraits': 'Chart traits',
    'common.recordTraits': 'Record traits',
    'common.close': 'Close',
    'common.connect': 'Connect',
    'common.connecting': 'Connecting...',
    'common.all': 'ALL',
    'common.apply': 'Apply',
    'common.search': 'Search',
    'common.loadingCharts': 'Loading charts...',
    'common.loadingPlaylogs': 'Loading playlogs...',
    'common.loadingVersions': 'Loading versions...',
    'common.jacket': 'Jacket',
    'common.compact': 'Compact',
    'common.title': 'Title',
    'common.chart': 'Chart',
    'common.levelShort': 'Lv',
    'common.achievementShort': 'Achv',
    'common.rating': 'Rating',
    'common.rank': 'Rank',
    'common.fc': 'FC',
    'common.sync': 'Sync',
    'common.dx': 'DX',
    'common.lastPlayed': 'Last Played',
    'common.playCount': 'Play Count',
    'common.version': 'Version',
    'common.track': 'Track',
    'common.type': 'Type',
    'common.diff': 'Diff',
    'common.achievement': 'Achievement',
    'common.error': 'Error',
    'common.min': 'MIN',
    'common.max': 'MAX',
    'common.from': 'FROM',
    'common.to': 'TO',
    'common.none': 'None',
    'common.ready': 'Ready',
    'units.songs': '{{count}} song(s)',
    'units.credits': '{{count}} credit(s)',
    'units.daysAgo': '{{count}} days ago',
    'player.connected': 'Connected player',
    'player.totalPlayCount': 'Total play count',
    'home.connect.title': 'Connect Record Collector',
    'home.connect.description': 'Enter your Record Collector server URL and verify the connection.',
    'home.connect.serverUrl': 'Server URL',
    'home.connect.placeholder': 'https://your-server.example.com',
    'home.connect.failed': 'Connection failed: {{message}}',
    'home.connect.success': 'Connected. Player: {{name}}',
    'home.connect.goToScores': 'Go to Scores →',
    'home.intro.description': 'maistats is a record management app that connects to your personal collector server and keeps Scores, Playlogs, Rating, and Picker in one place.',
    'home.intro.helper': 'It assumes you run your own Record Collector. If this is your first visit, start with the setup guide.',
    'home.startCard.title': 'Start here',
    'home.startCard.body': 'If your collector is not ready yet, open the setup guide for the server run and connection flow.',
    'home.discordCard.title': 'Discord Bot',
    'home.discordCard.body': 'You can also look up scores and recent plays directly from the Discord Bot.',
    'home.openSetup': 'Record Collector setup guide',
    'home.guide.title': 'Server Setup Guide',
    'home.guide.description': 'If you do not have a Record Collector server yet, follow the steps below.',
    'home.guide.step1Title': 'Create a compose.yaml file',
    'home.guide.step1BodyA': 'Create a ',
    'home.guide.step1BodyB': ' file in the folder where you want to run the server, using the content below. Fill in ',
    'home.guide.step1BodyC': ' and ',
    'home.guide.step1BodyD': ' with your maimaidx-eng.com account credentials.',
    'home.guide.step2Title': 'Run Docker Compose',
    'home.guide.step2BodyA': 'On a machine with Docker installed, run the command below in the folder containing ',
    'home.guide.step2BodyB': '.',
    'home.guide.step2BodyC': 'On first launch, the image will be downloaded and the server will log in to maimaidx-eng.com. Once ready, the ',
    'home.guide.step2BodyD': ' endpoint returns 200.',
    'home.guide.step3Title': 'Expose the server (optional)',
    'home.guide.step3Body': 'If you want to connect from outside, expose the server through a public IP or domain and enter that address here. Tools like ngrok or Cloudflare Tunnel work well.',
    'home.guide.step4Title': 'Connect the URL',
    'home.guide.step4BodyA': 'Once the server is ready, enter the server URL above and click ',
    'home.guide.step4BodyB': '. On success, the app will move to the Scores page automatically.',
    'home.discord.title': 'Discord Bot',
    'home.discord.description': 'Add the maistats bot to your Discord server to look up scores and recent plays directly with ',
    'home.discord.descriptionTail': '.',
    'home.discord.addButton': 'Add Discord Bot',
    'home.footer.aliases': 'Song title aliases were imported with permission from ',
    'home.footer.aliasesTail': '.',
    'home.footer.parsing': 'Song parsing was implemented with reference to ',
    'home.footer.parsingTail': '.',
    'home.footer.source': 'The source code for maistats is available at ',
    'home.footer.sourceTail': '.',
    'home.footer.developer': 'Developer:',
    'home.footer.copyrightA': 'This site is a fan-made ',
    'home.footer.copyrightB': ' site built for personal score tracking. Copyright for the in-game content used here belongs to ',
    'home.footer.copyrightC': ' and ',
    'home.footer.copyrightOwners': 'the respective owners',
    'home.footer.copyrightD': '.',
    'settings.title': 'Connections',
    'settings.description': 'Manage the Song Info and Record Collector connection settings.',
    'settings.songInfoWarning': 'Do not change this unless you are debugging.',
    'settings.language.title': 'Language',
    'settings.language.description': 'Use your device language by default, or override it for the app.',
    'settings.language.label': 'App language',
    'settings.language.helperSystem': 'Following your device language. Current app language: {{language}}',
    'settings.language.helperManual': 'Current app language: {{language}}',
    'settings.language.optionSystem': 'System default',
    'settings.language.optionKo': 'Korean',
    'settings.language.optionEn': 'English',
    'settings.theme.title': 'Theme',
    'settings.theme.description': 'Choose the app color theme.',
    'settings.theme.label': 'Color theme',
    'settings.theme.optionSystem': 'System default',
    'settings.theme.optionLight': 'Light',
    'settings.theme.optionDark': 'Dark',
    'settings.recordCollector.success': 'Connected. Player: {{name}}',
    'settings.recordCollector.failed': 'Connection failed: {{message}}',
    'scores.resetAll': 'Reset all',
    'scores.searchLabel': 'Search (title/alias/version/level)',
    'scores.searchPlaceholder': 'Example: VERTeX, Vertex, PRiSM, 14+',
    'scores.chartType': 'Chart Type',
    'scores.difficulty': 'Difficulty',
    'scores.level': 'Level',
    'scores.levelMin': 'Minimum level',
    'scores.levelMax': 'Maximum level',
    'scores.score': 'Score',
    'scores.achievementMin': 'Minimum achievement',
    'scores.achievementMax': 'Maximum achievement',
    'scores.version': 'Version',
    'scores.daysSince': 'Days Since',
    'scores.daysMin': 'Minimum days since',
    'scores.daysMax': 'Maximum days since',
    'scores.chartsTitle': 'Charts',
    'scores.chartsDescription': 'Browse score data with chart metadata. Gray decimal digits indicate an estimated internal level.',
    'scores.layout': 'Charts layout',
    'scores.versionAll': 'ALL',
    'scores.versionNew': 'NEW',
    'scores.versionOld': 'OLD',
    'playlogs.searchLabel': 'Search (title/alias/time)',
    'playlogs.searchPlaceholder': 'Example: 2026/02/25, BUDDiES, Bad Apple',
    'playlogs.showAll': 'Show all playlogs',
    'playlogs.dayLabel': 'Play date (maimai day starts at 04:00)',
    'playlogs.summaryAll': 'All: {{songCount}} song(s)',
    'playlogs.summaryDay': '{{songCount}} song(s) · {{creditCount}} credit(s)',
    'playlogs.bestOnly': 'Show only best record per song/chart',
    'playlogs.newRecordOnly': 'Show only new records',
    'playlogs.layout': 'Playlogs layout',
    'playlogs.creditNumber': 'Credit #',
    'playlogs.playedAt': 'Played At',
    'playlogs.dayOption': '{{date}} ({{credits}} credits)',
    'rating.title': 'RATING',
    'rating.description': 'This is the total of your top 15 NEW songs and top 35 OLD songs. Ratings may be inaccurate for songs without a known internal level.',
    'rating.current': 'Current Rating',
    'rating.newTop15': 'NEW TOP 15',
    'rating.oldTop35': 'OLD TOP 35',
    'rating.avg': 'avg {{value}}',
    'rating.avgProjection': 'avg {{avg}}, ~{{projection}}',
    'rating.newDescription': 'Top 15 songs in the NEW category. Click a card to open Song Detail.',
    'rating.oldDescription': 'Top 35 songs in the OLD category. Click a card to open Song Detail.',
    'rating.openSongDetail': 'Open Song Detail for {{title}}',
    'picker.filterSettings': 'Filter Settings',
    'picker.selectAll': 'Select all',
    'picker.clearAll': 'Clear all',
    'picker.chooseVersions': 'Choose versions',
    'picker.lastPlayedDays': 'Last played (days)',
    'picker.songInfoRequired': 'Song Info URL is empty.',
    'picker.noSongs': 'No songs match the current filters.',
    'picker.noSongsHelp': 'No songs match the current filters. Broaden the range or relax the filters.',
    'picker.picking': 'Picking...',
    'picker.pickFailed': 'Random pick failed',
    'picker.noData': 'No Data',
    'picker.unplayed': 'UNPLAYED',
    'picker.fcEmpty': 'FC -',
    'picker.syncEmpty': 'SYNC -',
    'picker.metaWithData': 'Last played: {{lastPlayed}}  •  Play count: {{playCount}}',
    'picker.metaNoData': 'Last played: No Data  •  Play count: No Data',
    'picker.pickedFrom': 'Picked from {{count}} songs',
    'picker.pickedFromNoData': 'Song pool unavailable',
    'picker.versionsSelected': '{{count}} versions selected',
    'picker.controlTitle': 'Picker Controls',
    'picker.filters': 'Filters',
    'picker.random': 'RANDOM',
    'picker.selection': 'Selection',
    'picker.statusPicking': 'Picking',
    'picker.statusError': 'Error',
    'picker.statusEmpty': 'Empty',
    'picker.statusPicked': 'Picked',
    'picker.statusReady': 'Ready',
    'songDetail.title': 'Song Detail',
    'songDetail.refreshing': 'Refreshing...',
    'songDetail.refresh': 'Refresh Score',
    'songDetail.refreshUnavailable': 'Cannot refresh because the song identifiers are incomplete.',
    'songDetail.empty': 'No detail data is available for this song.',
    'history.title': 'History',
    'history.description': 'Only points where the best achievement improved are shown, based on playlogs.',
    'history.loading': 'Loading playlogs for this chart.',
    'history.empty': 'No history entries for this chart were found in the playlogs.',
    'history.graphLabel': 'Achievement history graph for {{title}}',
    'history.axisAchievement': 'Achievement',
    'history.axisTime': 'Time',
    'app.missingUrls': 'Scores and Playlogs require both the Song Info URL and the Record Collector URL.',
    'api.enterUrl': 'Please enter a URL.',
    'api.connectionFailed': 'Received HTTP {{status}} from the server.',
    'api.recordCollectorRequired': 'Record Collector URL is empty.',
  },
});

export type TranslationKey = keyof typeof translations.ko;
export type TranslationVariables = Record<string, string | number>;

export function interpolate(template: string, variables?: TranslationVariables): string {
  if (!variables) {
    return template;
  }
  return template.replace(/\{\{(\w+)\}\}/g, (_, key: string) => {
    const value = variables[key];
    return value === undefined ? `{{${key}}}` : String(value);
  });
}

export function detectSystemLanguage(): SupportedLanguage {
  if (typeof navigator === 'undefined') {
    return 'en';
  }

  const candidates = [
    ...(Array.isArray(navigator.languages) ? navigator.languages : []),
    navigator.language,
  ].filter((value): value is string => Boolean(value));

  for (const value of candidates) {
    if (value.toLowerCase().startsWith('ko')) {
      return 'ko';
    }
  }

  return 'en';
}

export function normalizeLanguagePreference(value: string | null): LanguagePreference {
  if (value === 'ko' || value === 'en' || value === 'system') {
    return value;
  }
  return 'system';
}

interface I18nContextValue {
  languagePreference: LanguagePreference;
  setLanguagePreference: (value: LanguagePreference) => void;
  language: SupportedLanguage;
  locale: string;
  t: (key: TranslationKey, variables?: TranslationVariables) => string;
  formatNumber: (value: number) => string;
  compareText: (left: string, right: string) => number;
  formatLanguageName: (value: SupportedLanguage) => string;
}

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: PropsWithChildren) {
  const [languagePreference, setLanguagePreferenceState] = useState<LanguagePreference>(() => {
    if (typeof localStorage === 'undefined') {
      return 'system';
    }
    return normalizeLanguagePreference(localStorage.getItem(LANGUAGE_STORAGE_KEY));
  });
  const [systemLanguage, setSystemLanguage] = useState<SupportedLanguage>(() => detectSystemLanguage());

  const language = languagePreference === 'system'
    ? systemLanguage
    : languagePreference;
  const locale = LOCALE_BY_LANGUAGE[language];

  useEffect(() => {
    localStorage.setItem(LANGUAGE_STORAGE_KEY, languagePreference);
  }, [languagePreference]);

  useEffect(() => {
    if (typeof window === 'undefined') {
      return undefined;
    }

    const handleLanguageChange = () => {
      setSystemLanguage(detectSystemLanguage());
    };

    window.addEventListener('languagechange', handleLanguageChange);
    return () => {
      window.removeEventListener('languagechange', handleLanguageChange);
    };
  }, []);

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  const t = useCallback(
    (key: TranslationKey, variables?: TranslationVariables) => {
      return interpolate(translations[language][key], variables);
    },
    [language],
  );

  const formatNumber = useCallback(
    (value: number) => value.toLocaleString(locale),
    [locale],
  );

  const compareText = useCallback(
    (left: string, right: string) => left.localeCompare(right, locale),
    [locale],
  );

  const formatLanguageName = useCallback(
    (value: SupportedLanguage) => (value === 'ko' ? t('settings.language.optionKo') : t('settings.language.optionEn')),
    [t],
  );

  const value = useMemo<I18nContextValue>(
    () => ({
      languagePreference,
      setLanguagePreference: setLanguagePreferenceState,
      language,
      locale,
      t,
      formatNumber,
      compareText,
      formatLanguageName,
    }),
    [compareText, formatLanguageName, formatNumber, language, languagePreference, locale, t],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
  const value = useContext(I18nContext);
  if (value === null) {
    throw new Error('useI18n must be used within I18nProvider');
  }
  return value;
}

import { useI18n } from '../app/i18n';

export function HomeFooter() {
  const { t } = useI18n();

  return (
    <footer className="home-footer">
      <ul className="home-footer-credits">
        <li>
          {t('home.footer.aliases')}
          <a href="https://github.com/lomotos10/GCM-bot" target="_blank" rel="noreferrer">
            GCM-bot
          </a>
          {t('home.footer.aliasesTail')}
        </li>
        <li>
          {t('home.footer.parsing')}
          <a href="https://github.com/zetaraku/arcade-songs-fetch" target="_blank" rel="noreferrer">
            arcade-songs-fetch
          </a>
          {t('home.footer.parsingTail')}
        </li>
        <li>
          {t('home.footer.source')}
          <a href="https://github.com/minty99/maistats" target="_blank" rel="noreferrer">
            github.com/minty99/maistats
          </a>
          {t('home.footer.sourceTail')}
        </li>
        <li>
          {t('home.footer.developer')}{' '}
          <a href="https://github.com/minty99" target="_blank" rel="noreferrer">
            github.com/minty99
          </a>
        </li>
      </ul>
      <p className="home-footer-copyright">
        {t('home.footer.copyrightA')}
        <a href="https://maimai.sega.com/" target="_blank" rel="noreferrer">
          maimai DX
        </a>
        {t('home.footer.copyrightB')}
        <a href="https://www.sega.com/" target="_blank" rel="noreferrer">
          SEGA
        </a>{' '}
        {t('home.footer.copyrightC')}
        <a href="https://maimai.sega.com/song/new/#copy--list" target="_blank" rel="noreferrer">
          {t('home.footer.copyrightOwners')}
        </a>
        {t('home.footer.copyrightD')}
      </p>
    </footer>
  );
}

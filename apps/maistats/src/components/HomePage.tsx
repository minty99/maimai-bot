import type { ReactNode } from 'react';

import { useI18n } from '../app/i18n';
import logoUrl from '../assets/logo.png';
import { HomeFooter } from './HomeFooter';

const DISCORD_OAUTH_URL =
  'https://discord.com/oauth2/authorize?client_id=1463175635974361183';

interface HomePageProps {
  sidebarTopContent?: ReactNode;
  onNavigateToSetup: () => void;
}

export function HomePage({ sidebarTopContent, onNavigateToSetup }: HomePageProps) {
  const { t } = useI18n();

  return (
    <div className="explorer-layout">
      <aside className="sidebar-column">{sidebarTopContent}</aside>

      <div className="table-column home-content">
        <section className="panel home-intro-panel">
          <div className="home-hero">
            <div className="home-hero-identity">
              <img src={logoUrl} alt="" className="home-logo" aria-hidden="true" />
              <span className="home-wordmark">maistats</span>
            </div>
            <h2 className="home-tagline">{t('home.intro.description')}</h2>
          </div>
        </section>

        <div className="home-cards">
          <article className="home-card">
            <span className="home-card-label">{t('home.startCard.title')}</span>
            <p>{t('home.startCard.body')}</p>
            <button type="button" className="home-card-link" onClick={onNavigateToSetup}>
              {t('home.openSetup')}
            </button>
          </article>

          <article className="home-card">
            <span className="home-card-label">{t('home.discordCard.title')}</span>
            <p>{t('home.discordCard.body')}</p>
            <a
              href={DISCORD_OAUTH_URL}
              target="_blank"
              rel="noreferrer"
              className="home-card-link home-card-link-discord"
            >
              {t('home.discord.addButton')}
            </a>
          </article>
        </div>

        <HomeFooter />
      </div>
    </div>
  );
}

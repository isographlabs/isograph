import Buttons from '../Buttons';
import styles from './styles.module.css';
import clsx from 'clsx';

export default function HomepageHeader() {
  return (
    <header className={clsx('hero hero--primary', styles.heroBanner)}>
      <div className="container">
        <div className="row">
          <div className="col">
            <h1 className={styles.heroTitle}>Isograph</h1>
            <p
              className={clsx(
                'hero__subtitle margin-bottom--lg',
                styles.heroSubtitle,
              )}
            >
              The UI framework for teams that&nbsp;move&nbsp;fast&nbsp;&mdash;
              <br />
              <i>without</i> breaking things
            </p>
            <Buttons />
          </div>
        </div>
      </div>
    </header>
  );
}

import Link from '@docusaurus/Link';
import styles from './styles.module.css';

export default function () {
  return (
    <div className={styles.buttons}>
      <Link
        className="button button--secondary button--lg"
        to="/docs/quickstart"
      >
        Quickstart
      </Link>
      <Link
        className="button button--secondary button--lg"
        to="https://github.com/isographlabs/isograph/tree/main/demos/pet-demo"
      >
        See a demo app
      </Link>
    </div>
  );
}

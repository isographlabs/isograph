import { useEffect, useState } from 'react';

/**
 * This is a function that will render a component only after it commits.
 * It should not be used in production. It's useful as a way to debug issues
 * with NextJS, where an indefinite suspense causes the server to hang
 * forever and never complete the original request.
 */
export function RenderAfterCommit__DO_NOT_USE({
  children,
}: {
  children: React.ReactNode;
}) {
  const [show, setShow] = useState(false);
  useEffect(() => setShow(true), []);
  return show ? children : null;
}

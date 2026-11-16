// Healthcare's own interface at /healthcare/: the day's appointments by practitioner, with every
// record in the records frame's lists and forms.

import { useEffect } from "react";
import { Link, Outlet, useRouteError } from "react-router";
import { bootstrap, shell, t } from "@ferrum/client";
import { EmptyState, ErrorState } from "@ferrum/kit";
import { mountApp, RequireAuth, setFrameMode } from "@ferrum/records/frame";
import "./styles.css";

const page = (name: "FormView" | "ListView" | "ReportView") => () => import("./records").then((m) => ({ Component: m[name] }));

function RouteError() {
  return <ErrorState error={useRouteError()} onRetry={() => window.location.reload()} />;
}

function NotFound() {
  return (
    <EmptyState title={t("Page not found")}>
      <Link to="/">{t("Go to the day")}</Link>
    </EmptyState>
  );
}

function Started() {
  useEffect(() => shell().ready(), []);
  return null;
}

/** A page outside the frame: started, and drawn below. */
function Bare() {
  useEffect(() => shell().ready(), []);
  return <Outlet />;
}

/** The patient's page of the portal, when the platform serves this release there. */
const VISITS = "portal/visits";
const onPortal = bootstrap()?.app?.entry === VISITS;

if (!onPortal) setFrameMode("app", ["healthcare"]);

const portalRoutes = [
  {
    path: "/",
    element: (
      <main>
        <Bare />
      </main>
    ),
    errorElement: <RouteError />,
    children: [{ index: true, lazy: () => import("./patient/Visits").then((m) => ({ Component: m.VisitsPage })) }],
  },
];

const staffRoutes = [
  // For patients booking: no sign-in, no frame.
  { path: "/book/:tenant", element: <Bare />, errorElement: <RouteError />, children: [{ index: true, lazy: () => import("./patient/Book").then((m) => ({ Component: m.BookPage })) }] },
  {
    path: "/book/:tenant/visit/:token",
    element: <Bare />,
    errorElement: <RouteError />,
    children: [{ index: true, lazy: () => import("./patient/Book").then((m) => ({ Component: m.VisitPage })) }],
  },
  { path: "/book/*", element: <EmptyState title={t("Page not found")} icon="calendar" /> },
    {
      path: "/",
      element: (
        <>
          <Started />
          <RequireAuth />
        </>
      ),
      errorElement: <RouteError />,
      children: [
        { index: true, lazy: () => import("./Home").then((m) => ({ Component: m.Home })) },
        { path: "e/:entity", lazy: page("ListView") },
        { path: "e/:entity/:id", lazy: page("FormView") },
        { path: "r/:name", lazy: page("ReportView") },
        { path: "*", element: <NotFound /> },
      ],
    },
];

mountApp({
  catalogs: { ar: () => import("./locales/ar.json"), hi: () => import("./locales/hi.json") },
  ...(onPortal ? { route: VISITS, routes: portalRoutes } : { route: "healthcare", routes: staffRoutes }),
});

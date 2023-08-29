import Image from "next/image";
import { iso } from "@isograph/react";
import { ResolverParameterType as HeaderProps } from "./__isograph/Query/header.isograph";

import { AppBar, Button, Grid, Container } from "@mui/material";
import { useTheme } from "@mui/material/styles";
import { Route } from "./github_demo";

import logo from "./svgs/dark-logo.svg";

export const header = iso<HeaderProps, ReturnType<typeof Header>>`
  Query.header @component {
    viewer {
      name,
      avatar,
    },
  }
`(Header);

function Header(props: HeaderProps) {
  return (
    <>
      <AppBar position="fixed" color="primary" sx={{ p: 0.5 }}>
        <Container maxWidth="md">
          <Grid
            container
            spacing={24}
            justifyContent="space-between"
            maxWidth="md"
          >
            <Grid item xs={6} style={{ flex: 1 }}>
              <Buttons route={props.route} setRoute={props.setRoute} />
            </Grid>
            <Grid item xs={6}>
              <div style={{ display: "flex", justifyContent: "flex-end" }}>
                {props.data.viewer.avatar({})}
              </div>
            </Grid>
          </Grid>
        </Container>
      </AppBar>
      <div style={{ height: 48 }} />
    </>
  );
}

function Buttons({
  route,
  setRoute,
}: {
  route: Route;
  setRoute: (route: Route) => void;
}) {
  const theme = useTheme();
  return (
    <>
      <div style={{ display: "flex" }}>
        <a href="https://github.com/isographlabs/isograph">
          <Image src={logo} alt="Isograph Logo" height={40} width={40} />
        </a>
        <Button
          variant="text"
          style={{
            color: theme.palette.primary.contrastText,
            textDecoration: route.kind === "Home" ? "underline" : "none",
          }}
          onClick={() => setRoute({ kind: "Home" })}
        >
          Home
        </Button>
      </div>
    </>
  );
}

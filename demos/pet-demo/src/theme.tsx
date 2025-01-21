import { createTheme, ThemeProvider } from '@mui/material/styles';
import React from 'react';

const theme = createTheme({
  spacing: 4,
  palette: {
    primary: {
      light: '#788caf',
      main: '#385276',
      dark: '#1a2f4a',
      contrastText: '#fff',
    },
    secondary: {
      light: '#ff7961',
      main: '#f28800',
      dark: '#e86600',
      contrastText: '#000',
    },
  },
});

export default function PetDemoThemeProvider({
  children,
}: any): React.ReactNode {
  return <ThemeProvider theme={theme}>{children}</ThemeProvider>;
}

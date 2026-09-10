import { createTheme} from "@mui/material";
import { amber, blue } from "@mui/material/colors";

const theme = createTheme({
  palette: {
    mode: "dark",
    primary: amber,
    secondary: blue,
    background: {
      default: "#303030",
    },
  },
  typography: {
    h1: {
      fontSize: "2.25rem"
    }
  },
  components: {
    MuiOutlinedInput: {
      styleOverrides: {
        // Safari/WebKit (the WKWebView Tauri uses on macOS) doesn't size the notch that the
        // outline leaves for the floating label: it treats the outline's `<legend>` as if it
        // contributed nothing to layout because the legend is `visibility: hidden` (its actual
        // glyphs are hidden via `opacity: 0` on a child span instead, purely so the legend still
        // reserves width). Chromium and Firefox size the notch from that hidden legend correctly;
        // WebKit collapses it to zero width, so the border is drawn straight through the label.
        // Forcing the legend itself back to `visibility: visible` fixes WebKit's sizing while the
        // child span's `opacity: 0` still keeps its glyphs unpainted, so nothing doubles up.
        // Confirmed fix from https://github.com/mui/material-ui/issues/46891 (WebKit-only via
        // an `-webkit-appearance` feature query, since Chromium/Firefox already work).
        notchedOutline: {
          "@supports (-webkit-appearance: none)": {
            "& legend": {
              visibility: "visible !important",
            },
          },
        },
      },
    },
  },
});


export default theme;

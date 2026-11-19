import buttonStyles from './codeButtonStyles.css?raw';
import {codeButtons} from './codeLibraryData';
const escapeHtml=(value:string)=>value.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
/** Exports the same authored CSS and SVG markup used by the live preview. */
export function exportCodeButton(id:string,element:HTMLButtonElement):string{
 const component=codeButtons.find(button=>button.id===id);
 if(!component||!element.classList.contains(`code-demo-${id}`))throw Error('Component preview is unavailable.');
 const style=getComputedStyle(element),stage=element.closest('.code-specimen-stage');
 if(!stage)throw Error('Component preview is unavailable.');
 const backdrop=getComputedStyle(stage);
 // Keep the font and preview background used at copy time, independent of the app theme.
 const font=style.fontFamily.replaceAll('<','\\3c ');
 const sheet=new CSSStyleSheet();sheet.replaceSync(buttonStyles);
 const css=[...sheet.cssRules].filter(rule=>!(rule instanceof CSSStyleRule)||!codeButtons.some(button=>rule.selectorText.includes(`.code-demo-${button.id}`))||rule.selectorText.includes(`.code-demo-${id}`)).map(rule=>rule.cssText).join('\n');
 return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(component.name)} button</title>
<style>
/* ${component.name}: the exact live-preview styles. No framework or icon package required.
   Hover, keyboard focus, pressed state and reduced motion are included.
   Font stack is captured from the app; install those fonts for the same typography. */
* { box-sizing: border-box; }
body { margin: 0; min-height: 100vh; display: grid; place-items: center; background: ${backdrop.backgroundColor}; }
.code-library { font-family: ${font}; font-synthesis: ${style.fontSynthesis}; -webkit-font-smoothing: ${style.getPropertyValue('-webkit-font-smoothing')||'auto'}; }
.code-specimen-stage { display: flex; align-items: center; justify-content: center; min-height: 260px; padding: 40px 24px; background-color: ${backdrop.backgroundColor}; background-image: ${backdrop.backgroundImage}; }
${css}
</style>
</head>
<body>
<main class="code-library">
  <div class="code-specimen-stage">
    ${element.outerHTML}
  </div>
</main>
<script>
// No JavaScript is needed: all interactions are implemented in CSS.
// Add your application's click handler here when connecting this button to an action.
</script>
</body>
</html>`;
}

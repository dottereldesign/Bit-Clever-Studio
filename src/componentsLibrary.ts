export const allComponents = "All Components";
const prefix = "Components / ";
export const componentInbox = prefix + "Uncategorised";

export const componentGroups: Record<string, string[]> = {
  "Site parts": ["Navbars", "Mega menus", "Headers", "Footers", "404 pages", "Dashboard", "Templates"],
  "Page sections": ["Features", "Calls to action", "Stats", "Testimonials", "Pricing", "FAQs", "Contact", "Teams", "Careers", "Sponsors"],
  "Content & media": ["Image boxes", "Icon boxes", "Text blocks", "Galleries", "Steps", "Timelines"],
  "Interactive": ["Buttons", "Sliders", "Tabs", "Accordions"],
  "Forms": ["Logins & Signups", "Email opt-ins"],
  "Blog & portfolio": ["Blog & News Sections", "Single posts", "Portfolio headers", "Portfolio sections"],
  "Shop": ["Products"],
};

export const componentKey = (folder: string) => prefix + folder;
export const isComponent = (name: string) => name.startsWith(prefix);
export const componentFolders = [componentInbox, ...Object.values(componentGroups).flat().map(componentKey)];
export const isWebDesign = (name: string) => /^web[\s_-]*design$/i.test(name.trim());
export const isHeaderFolder = (name:string) => /^Components \/ (Heroes|Hero|Headers|Header)$/.test(name);
export const isBlogFolder = (name:string) => /^Components \/ (Blog sections|Blog archives|Blog & News Sections)$/i.test(name);
export const isAuthFolder = (name:string) => /^Components \/ (Logins|Signups|Logins & Signups)$/i.test(name);
export const isProductsFolder = (name:string) => /^Components \/ (Products|Product grids)$/i.test(name);
export const isRetiredComponentFolder = (name:string) => /^Components \/ (Banners|Cart & checkout|Logo grids|Videos)$/i.test(name);
export const filingCategory = (name: string) => name === allComponents || isWebDesign(name) || isRetiredComponentFolder(name) ? componentInbox : isHeaderFolder(name) ? componentKey("Headers") : isBlogFolder(name) ? componentKey("Blog & News Sections") : isAuthFolder(name) ? componentKey("Logins & Signups") : isProductsFolder(name) ? componentKey("Products") : name;

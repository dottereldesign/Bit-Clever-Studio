import {useHistoryState} from './useAppHistory';
import {readHiddenSections} from './sidebarVisibilityData';

export function useSidebarVisibility(){
  return useHistoryState<string[]>('sidebar-hidden-sections',()=>readHiddenSections(localStorage.getItem('sidebar-hidden-sections')),'Sidebar section visibility');
}

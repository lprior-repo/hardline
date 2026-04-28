// Isolate Documentation - Collapsible Sidebar Navigation
// Inspired by Stripe's documentation UX

const initCollapsibleSections = () => {
  const chapters = document.querySelectorAll('.chapter > li.chapter-item');
  
  chapters.forEach((chapter) => {
    const link = chapter.querySelector(':scope > a');
    const sublist = chapter.querySelector(':scope > ul');
    
    if (sublist) {
      chapter.classList.add('has-children');
      
      const hasActive = sublist.querySelector('a.active');
      if (hasActive) {
        chapter.classList.add('expanded');
      } else {
        chapter.classList.add('collapsed');
      }
      
      if (link) {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          toggleSection(chapter);
        });
      }
    }
  });
};

const toggleSection = (chapter) => {
  const isExpanded = chapter.classList.contains('expanded');
  
  if (isExpanded) {
    chapter.classList.remove('expanded');
    chapter.classList.add('collapsed');
  } else {
    chapter.classList.remove('collapsed');
    chapter.classList.add('expanded');
  }
};

const highlightCurrentPage = () => {
  const currentPath = window.location.pathname;
  const links = document.querySelectorAll('.chapter a');
  
  links.forEach((link) => {
    const href = link.getAttribute('href');
    if (href && currentPath.endsWith(href.replace('./', ''))) {
      link.classList.add('active');
      
      let parent = link.closest('.chapter-item');
      while (parent) {
        parent.classList.remove('collapsed');
        parent.classList.add('expanded');
        parent = parent.parentElement.closest('.chapter-item');
      }
    }
  });
};

const restoreScrollPosition = () => {
  const sidebar = document.querySelector('.sidebar-scrollbox');
  if (!sidebar) return;
  
  const savedPosition = sessionStorage.getItem('isolate-sidebar-scroll');
  if (savedPosition) {
    sidebar.scrollTop = parseInt(savedPosition, 10);
  }
  
  sidebar.addEventListener('scroll', () => {
    sessionStorage.setItem('isolate-sidebar-scroll', sidebar.scrollTop);
  });
};

const init = () => {
  initCollapsibleSections();
  highlightCurrentPage();
  restoreScrollPosition();
};

document.addEventListener('DOMContentLoaded', init);

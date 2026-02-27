// ADI website — client-side entry point

// Initialize Lucide icons after DOM is ready
document.addEventListener("DOMContentLoaded", () => {
  lucide.createIcons();
});

// Language selector modal
function openLangModal() {
  const modal = document.getElementById("lang-modal");
  modal.classList.add("is-open");
  lucide.createIcons({ nameAttr: "data-lucide", attrs: {} });
}

function closeLangModal() {
  document.getElementById("lang-modal").classList.remove("is-open");
}

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") closeLangModal();
});

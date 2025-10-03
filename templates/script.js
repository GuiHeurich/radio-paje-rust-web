// Add animation on scroll
const cards = document.querySelectorAll(".top-card");
const observer = new IntersectionObserver(
  (entries) => {
    entries.forEach((entry, index) => {
      if (entry.isIntersecting) {
        setTimeout(() => {
          entry.target.style.opacity = "0";
          entry.target.style.transform = "translateY(20px)";
          entry.target.style.transition = "all 0.5s ease";

          setTimeout(() => {
            entry.target.style.opacity = "1";
            entry.target.style.transform = "translateY(0)";
          }, 100);
        }, index * 100);
      }
    });
  },
  { threshold: 0.1 },
);

cards.forEach((card) => {
  observer.observe(card);
});

// Add WebSocket functionality
const ws = new WebSocket("ws://127.0.0.1:8080/echo");
const messages = document.getElementById("messages");
const input = document.getElementById("input");
ws.onmessage = (event) => {
  const messageDiv = document.createElement("div");
  messageDiv.className = "message";
  messageDiv.textContent = event.data;
  messages.appendChild(messageDiv);
  messages.scrollTop = messages.scrollHeight;
};
input.addEventListener("keypress", (e) => {
  if (e.key === "Enter" && input.value) {
    ws.send(input.value);
    input.value = "";
  }
});

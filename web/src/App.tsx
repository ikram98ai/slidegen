import { useEffect } from "react";
import { Routes, Route, useNavigate } from "react-router-dom";
import { Header } from "./components/Header";
import { useAuthStore } from "./store/authStore";
import { useUIStore } from "./store/uiStore";
import { AuthScreen } from "./components/AuthScreen";
import { Button } from "./components/Button";
import { FilesPage } from "./pages/FilesPage";
import { UploadPage } from "./pages/UploadPage";
import { ProfilePage } from "./pages/ProfilePage";
import { ReaderPage } from "./pages/ReaderPage";

function App() {
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const user = useAuthStore((state) => state.user);
  const { showAuthModal, setShowAuthModal } = useUIStore();
  const navigate = useNavigate();

  // Close auth modal when authenticated
  useEffect(() => {
    if (isAuthenticated) {
      setShowAuthModal(false);
    }
  }, [isAuthenticated, setShowAuthModal]);

  if (showAuthModal) {
    return (
      <div className="relative">
        <div className="absolute top-4 right-4 z-50">
          <Button variant="ghost" onClick={() => setShowAuthModal(false)}>
            Cancel
          </Button>
        </div>
        <AuthScreen />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-apple-gray font-sans text-apple-text selection:bg-blue-100 selection:text-blue-900">
      <Header
        isAuthenticated={isAuthenticated}
        userName={user?.full_name}
        dp={user?.dp}
        onLogoClick={() => navigate('/')}
        onSignInClick={() => setShowAuthModal(true)}
        onProfileClick={() => navigate('/profile')}
      />

      <main className="relative">
        <Routes>
          <Route path="/" element={<FilesPage />} />
          <Route path="/upload" element={<UploadPage />} />
          <Route path="/profile" element={<ProfilePage />} />
          <Route path="/subject/:id" element={<ReaderPage />} />
        </Routes>
      </main>
    </div>
  );
}

export default App;

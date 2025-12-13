import React, { useEffect } from "react";
import type { StoredDocument } from "./types";
import { DocType, Tab } from "./types";
import { NavBar } from "./components/NavBar";
import { useAuthStore } from "./store/authStore";
import { useUIStore } from "./store/uiStore";
import { useDocumentStore } from "./store/documentStore";
import { useViewerStore } from "./store/viewerStore";
import { AuthScreen } from "./components/AuthScreen";
import { ProfilePage } from "./components/ProfilePage";
import {
  useDocuments,
  useAddDocument,
  useDocumentDetails,
} from "./hooks/useAppQueries";
import { lessonsApi } from "./services/api";
import { Header } from "./components/Header";
import { UploadTab } from "./components/UploadTab";
import { DocumentList } from "./components/DocumentList";
import { BookReader } from "./components/BookReader";
import { ReportReader } from "./components/ReportReader";
import { Button } from "./components/Button";

function App() {
  // Auth Store
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);
  const user = useAuthStore((state) => state.user);

  // UI Store
  const {
    activeTab,
    showAuthModal,
    viewingDocId,
    setActiveTab,
    setShowAuthModal,
    setViewingDocId,
  } = useUIStore();

  // Document Store (Upload)
  const {
    file,
    docType,
    error,
    setFile,
    setFileBase64,
    setIsAnalyzing,
    setError,
    reset: resetUpload,
  } = useDocumentStore();

  // Viewer Store
  const {
    activeChapterId,
    viewMode,
    currentHorizontalIndex,
    setActiveChapterId,
    setViewMode,
    setCurrentHorizontalIndex,
    reset: resetViewer,
  } = useViewerStore();

  // React Query Hooks
  const { data: documentsList, isLoading: isLoadingDocs } = useDocuments();
  const { data: documentDetails } = useDocumentDetails(viewingDocId);

  const addDocumentMutation = useAddDocument();

  // Derived State
  // Merge list data with details
  const activeDocument = React.useMemo(() => {
    if (!viewingDocId) return null;
    const listDoc = documentsList?.find((d) => d.id === viewingDocId);
    if (!listDoc && !documentDetails) return null;

    // If we have details, use them. Otherwise use list doc (which might be partial)
    if (documentDetails) {
      // If details title is "Loading...", try to use list title
      if (documentDetails.title === "Loading..." && listDoc) {
        return { ...documentDetails, title: listDoc.title, type: listDoc.type };
      }
      return documentDetails;
    }
    return listDoc || null;
  }, [viewingDocId, documentsList, documentDetails]);

  // Set active chapter when document loads
  useEffect(() => {
    if (
      activeDocument &&
      activeDocument.type === DocType.BOOK &&
      activeDocument.chapters.length > 0 &&
      !activeChapterId
    ) {
      setActiveChapterId(activeDocument.chapters[0].id);
    }
  }, [activeDocument, activeChapterId, setActiveChapterId]);

  // Close auth modal when authenticated
  useEffect(() => {
    if (isAuthenticated) {
      setShowAuthModal(false);
    }
  }, [isAuthenticated, setShowAuthModal]);

  // --- ACTIONS ---

  const handleFileSelect = (selectedFile: File) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const result = e.target?.result as string;
      const base64 = result.split(",")[1];

      setFile(selectedFile);
      setFileBase64(base64);
      setError(null);
    };
    reader.readAsDataURL(selectedFile);
  };

  const handleStartAnalysis = async () => {
    if (!file) return;

    setIsAnalyzing(true);
    setError(null);

    try {
      const newDoc: StoredDocument = {
        id: Date.now().toString(), // Temporary ID, will be replaced by backend
        title: file.name.replace(".pdf", ""),
        type: docType,
        uploadDate: new Date(),
        fileBase64: null,
        file: file,
        chapters: [],
        reportSlides: [],
        isUserOwner: true,
        author: "You",
      };

      // Save to "Database" via Mutation
      addDocumentMutation.mutate(newDoc, {
        onSuccess: () => {
          // Reset Upload State
          resetUpload();
          setActiveTab(Tab.FILES);
        },
        onError: () => {
          setIsAnalyzing(false);
          setError("Failed to save document.");
        },
      });
    } catch {
      setIsAnalyzing(false);
      setError("We encountered an issue analyzing your document.");
    }
  };

  const openDocument = (doc: StoredDocument) => {
    setViewingDocId(doc.id);
    resetViewer(); // Reset viewer state for new document
  };

  const closeDocument = () => {
    setViewingDocId(null);
    resetViewer();
  };

  const generateSlidesForActiveChapter = async () => {
    if (!activeDocument || !activeChapterId) return;

    // Find lesson ID
    // activeChapterId is the lesson ID (string)
    const lessonId = parseInt(activeChapterId);
    if (isNaN(lessonId)) return;

    try {
      await lessonsApi.generateSlides(lessonId);
      alert("Slides generation started. Please check back in a few moments.");
    } catch {
      alert("Failed to start slides generation.");
    }
  };

  // --- VIEWS ---

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
        onLogoClick={() => {
          setViewingDocId(null);
          setActiveTab(Tab.FILES);
        }}
        onSignInClick={() => setShowAuthModal(true)}
        onProfileClick={() => setActiveTab(Tab.PROFILE)}
      />

      <main className="relative">
        {/* If viewing a specific doc, take over the screen, otherwise show tabs */}
        {activeDocument ? (
          activeDocument.type === DocType.BOOK ? (
            <BookReader
              doc={activeDocument}
              activeViewerChapterId={activeChapterId}
              setActiveViewerChapterId={setActiveChapterId}
              viewMode={viewMode}
              setViewMode={setViewMode}
              currentHorizontalIndex={currentHorizontalIndex}
              setCurrentHorizontalIndex={setCurrentHorizontalIndex}
              onClose={closeDocument}
              onGenerateSlides={generateSlidesForActiveChapter}
            />
          ) : (
            <ReportReader
              doc={activeDocument}
              viewMode={viewMode}
              setViewMode={setViewMode}
              currentHorizontalIndex={currentHorizontalIndex}
              setCurrentHorizontalIndex={setCurrentHorizontalIndex}
              onClose={closeDocument}
            />
          )
        ) : (
          <>
            <div className="pt-8">
              <NavBar />
            </div>

            {activeTab === Tab.UPLOAD && (
              <UploadTab
                onFileSelect={handleFileSelect}
                onStartAnalysis={handleStartAnalysis}
                isAnalyzing={addDocumentMutation.isPending} // Use mutation pending state as well
              />
            )}

            {activeTab === Tab.FILES && (
              <DocumentList
                docs={documentsList}
                isLoading={isLoadingDocs}
                emptyMsg="No public documents found."
                onDocumentClick={openDocument}
                showUploadPrompt={false}
                onUploadClick={() => setActiveTab(Tab.UPLOAD)}
              />
            )}

            {activeTab === Tab.PROFILE &&
              (isAuthenticated ? (
                <ProfilePage onDocumentClick={openDocument} />
              ) : null)}
          </>
        )}
      </main>

      {/* Global Error Toast from Document Store */}
      {error && (
        <div className="fixed bottom-6 right-6 bg-red-500 text-white px-6 py-4 rounded-xl shadow-2xl flex items-center z-50 animate-bounce-in">
          <span className="font-medium mr-2">Error:</span> {error}
          <button
            onClick={() => setError(null)}
            className="ml-4 opacity-75 hover:opacity-100"
          >
            ✕
          </button>
        </div>
      )}
    </div>
  );
}

export default App;

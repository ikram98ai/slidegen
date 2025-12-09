import type { User, StoredDocument, Slide } from '../types';
import { DocType } from '../types';

// Initial Dummy Data
const INITIAL_DOCS: StoredDocument[] = [
  {
    id: 'demo-1',
    title: 'Introduction to Quantum Mechanics',
    type: DocType.BOOK,
    uploadDate: new Date('2023-10-15'),
    author: 'Dr. Feynman',
    isUserOwner: false,
    reportSlides: [],
    chapters: [
      { 
        id: 'c1', 
        title: 'Wave-Particle Duality', 
        description: 'Exploring the fundamental nature of light and matter.', 
        isLoadingSlides: false,
        slides: [
            { 
              title: "Light as a Wave", 
              bullets: ["Interference patterns observed", "Diffraction through slits", "Maxwell's equations"],
              explanation: "Historically, light was understood purely as a wave, a concept solidified by Young's double-slit experiment which demonstrated unmistakable interference patterns. Maxwell later unified electricity and magnetism, mathematically describing light as electromagnetic waves."
            },
            { 
              title: "Light as a Particle", 
              bullets: ["Photoelectric effect", "Discrete energy packets", "Einstein's proposal"],
              explanation: "The wave theory collapsed when it couldn't explain the photoelectric effect. Einstein proposed that light consists of discrete packets of energy called photons. This idea earned him the Nobel Prize and established the particle nature of light."
            }
        ] 
      },
      { id: 'c2', title: 'The Uncertainty Principle', description: 'Heisenberg and the limits of precision.', slides: [] }
    ]
  },
  {
    id: 'demo-2',
    title: 'Q3 Financial Performance Report',
    type: DocType.REPORT,
    uploadDate: new Date('2023-11-02'),
    author: 'Sarah Connors',
    isUserOwner: false,
    chapters: [],
    reportSlides: [
      { 
        title: "Executive Summary", 
        bullets: ["Revenue up 15% YoY", "New market entry successful", "Cost reduction initiatives on track"],
        explanation: "We are pleased to report a strong third quarter. Our year-over-year revenue has increased by 15%, largely driven by our successful expansion into the Asian market. Furthermore, our operational efficiency programs are delivering results ahead of schedule."
      },
      { 
        title: "Regional Analysis", 
        bullets: ["North America: Strong growth", "Europe: Stable performance", "Asia: Emerging opportunities"],
        explanation: "North American operations continue to be our bedrock, showing robust double-digit growth. Europe remains stable despite economic headwinds. However, Asia is our star performer this quarter, exceeding our initial penetration targets by 20%."
      }
    ]
  }
];

// Helper to simulate DB delay
export const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

class MockBackend {
  private users: User[] = [];
  private documents: StoredDocument[] = [];

  constructor() {
    this.loadFromStorage();
  }

  private loadFromStorage() {
    const storedUsers = localStorage.getItem('lumina_users');
    const storedDocs = localStorage.getItem('lumina_docs');

    if (storedUsers) {
      this.users = JSON.parse(storedUsers);
    } else {
        // Create a default demo user
        this.users = [{
            id: 'user-1',
            name: 'Demo User',
            email: 'demo@lumina.com',
            avatar: 'https://ui-avatars.com/api/?name=Demo+User&background=0D8ABC&color=fff'
        }];
    }

    if (storedDocs) {
      this.documents = JSON.parse(storedDocs, (key, value) => {
          if (key === 'uploadDate') return new Date(value);
          return value;
      });
    } else {
      this.documents = INITIAL_DOCS;
    }
  }

  private saveToStorage() {
    localStorage.setItem('lumina_users', JSON.stringify(this.users));
    localStorage.setItem('lumina_docs', JSON.stringify(this.documents));
  }

  // --- Auth ---

  async login(email: string): Promise<User> {
    await delay(800);
    const user = this.users.find(u => u.email === email);
    if (!user) throw new Error("User not found");
    return user;
  }

  async register(name: string, email: string, avatar: string): Promise<User> {
    await delay(1000);
    if (this.users.find(u => u.email === email)) throw new Error("Email already exists");
    
    const newUser: User = {
      id: `user-${Date.now()}`,
      name,
      email,
      avatar: avatar || `https://ui-avatars.com/api/?name=${encodeURIComponent(name)}&background=random`
    };
    
    this.users.push(newUser);
    this.saveToStorage();
    return newUser;
  }

  // --- Documents ---

  async getDocuments(): Promise<StoredDocument[]> {
    await delay(500);
    return [...this.documents];
  }

  async addDocument(doc: StoredDocument): Promise<StoredDocument> {
    await delay(800);
    this.documents.unshift(doc);
    this.saveToStorage();
    return doc;
  }

  async updateSlide(docId: string, chapterId: string | undefined, slideIndex: number, updatedSlide: Slide): Promise<void> {
    await delay(600);
    const docIndex = this.documents.findIndex(d => d.id === docId);
    if (docIndex === -1) throw new Error("Document not found");

    const doc = this.documents[docIndex];

    if (chapterId) {
        // Book Chapter Slide
        const chapterIndex = doc.chapters.findIndex(c => c.id === chapterId);
        if (chapterIndex === -1) throw new Error("Chapter not found");
        
        const slides = doc.chapters[chapterIndex].slides;
        if (!slides) throw new Error("Slides not initialized");

        slides[slideIndex] = updatedSlide;
    } else {
        // Report Slide
        doc.reportSlides[slideIndex] = updatedSlide;
    }

    this.saveToStorage();
  }
}

export const mockDb = new MockBackend();
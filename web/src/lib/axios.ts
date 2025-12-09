import axios from 'axios';
import { mockDb } from './mockBackend';

const api = axios.create({
  baseURL: '/api', // Dummy base URL
  headers: {
    'Content-Type': 'application/json',
  },
});

// Request Interceptor (Mocking the network)
api.interceptors.request.use(async (config) => {
  // Simulate network delay logic is inside mockDb methods, 
  // but we intercept the HTTP call here to route to our class
  
  const { url, method, data, params } = config;

  // --- Auth Routes ---
  if (url === '/auth/login' && method === 'post') {
    return new Promise((resolve, reject) => {
        mockDb.login(data.email)
            .then(user => resolve({ data: { user, token: 'fake-jwt-token' }, status: 200, statusText: 'OK', headers: {}, config }))
            .catch(err => reject({ response: { status: 401, data: { message: err.message } } }));
    });
  }

  if (url === '/auth/register' && method === 'post') {
    return new Promise((resolve, reject) => {
        mockDb.register(data.name, data.email, data.avatar)
            .then(user => resolve({ data: { user, token: 'fake-jwt-token' }, status: 201, statusText: 'Created', headers: {}, config }))
            .catch(err => reject({ response: { status: 400, data: { message: err.message } } }));
    });
  }

  // --- Document Routes ---
  if (url === '/documents' && method === 'get') {
    return new Promise((resolve) => {
        mockDb.getDocuments()
            .then(docs => resolve({ data: docs, status: 200, statusText: 'OK', headers: {}, config }));
    });
  }

  if (url === '/documents' && method === 'post') {
    return new Promise((resolve) => {
        mockDb.addDocument(data)
            .then(doc => resolve({ data: doc, status: 201, statusText: 'Created', headers: {}, config }));
    });
  }

  if (url === '/documents/slide' && method === 'put') {
      return new Promise((resolve, reject) => {
          mockDb.updateSlide(data.documentId, data.chapterId, data.slideIndex, data.slide)
            .then(() => resolve({ data: { success: true }, status: 200, statusText: 'OK', headers: {}, config }))
            .catch(err => reject({ response: { status: 400, data: { message: err.message } } }));
      });
  }

  // Pass through if not mocked (won't happen in this setup but good practice)
  return config;
}, (error) => {
  return Promise.reject(error);
});

// Response Interceptor to unwrap the promise returned by request interceptor
// Since we are returning a Promise in the request interceptor that resolves to a response object,
// axios actually treats this as an "adapter" replacement. 
// However, standard axios interceptors expect to modify config.
// To make this work seamlessly with standard axios usage:
// We will use a custom adapter.

api.defaults.adapter = async (config) => {
    const { url, method, data: dataStr } = config;
    const data = dataStr ? JSON.parse(dataStr) : {};

    // --- Auth Routes ---
    if (url === '/auth/login' && method === 'post') {
        try {
            const user = await mockDb.login(data.email);
            return { data: { user, token: 'fake-jwt' }, status: 200, statusText: 'OK', headers: {}, config };
        } catch (e: any) {
            throw { response: { status: 401, data: { message: e.message } } };
        }
    }

    if (url === '/auth/register' && method === 'post') {
        try {
            const user = await mockDb.register(data.name, data.email, data.avatar);
            return { data: { user, token: 'fake-jwt' }, status: 201, statusText: 'Created', headers: {}, config };
        } catch (e: any) {
            throw { response: { status: 400, data: { message: e.message } } };
        }
    }

    // --- Document Routes ---
    if (url === '/documents' && method === 'get') {
        const docs = await mockDb.getDocuments();
        return { data: docs, status: 200, statusText: 'OK', headers: {}, config };
    }

    if (url === '/documents' && method === 'post') {
        const doc = await mockDb.addDocument(data);
        return { data: doc, status: 201, statusText: 'Created', headers: {}, config };
    }

    if (url === '/documents/slide' && method === 'put') {
        try {
            await mockDb.updateSlide(data.documentId, data.chapterId, data.slideIndex, data.slide);
            return { data: { success: true }, status: 200, statusText: 'OK', headers: {}, config };
        } catch (e: any) {
            throw { response: { status: 400, data: { message: e.message } } };
        }
    }

    return { data: {}, status: 404, statusText: 'Not Found', headers: {}, config };
};

export default api;
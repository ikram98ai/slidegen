import React, { useState, useEffect } from 'react';
import { useAuthStore } from '../store/authStore';
import { usersApi } from '../services/api';
import { Button } from '../components/Button';
import { User, Mail, Save, Upload, Pencil } from 'lucide-react';
import { useUserSubjects } from '../hooks/useAppQueries';
import { useNavigate } from 'react-router-dom';
import { SubjectList } from '../components/SubjectList';
import { Modal } from '../components/Modal';
import { NavBar } from '../components/NavBar';


export const ProfilePage: React.FC = () => {
    const navigate = useNavigate();

  const { user, updateUser } = useAuthStore();
  const [name, setName] = useState(user?.full_name || '');
  const [email, setEmail] = useState(user?.email || '');
  const [isEditing, setIsEditing] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error', text: string } | null>(null);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<string | null>(null);

  const { data: subjects, isLoading: subjectsLoading } = useUserSubjects(user?.id);

  useEffect(() => {
    if (user) {
      setName(user.full_name);
      setEmail(user.email);
    }
  }, [user]);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      setSelectedFile(e.target.files[0]);
      setPreview(URL.createObjectURL(e.target.files[0]));
    }
  };

  const handleUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user) return;
    
    setIsLoading(true);
    setMessage(null);
    
    try {
      const formData = new FormData();
      const updateData: Record<string, string> = {};
      
      if (name !== user.full_name) updateData.full_name = name;
      if (email !== user.email) updateData.email = email;
      
      if (selectedFile) {
        formData.append('dp', selectedFile);
      }
      
      if (Object.keys(updateData).length === 0 && !selectedFile) {
          setIsLoading(false);
          setIsEditing(false);
          return;
      }

      formData.append('user_update', JSON.stringify(updateData));

      const updatedUser = await usersApi.updateUser(formData);
      
      // Update the store with the new user data
      updateUser(updatedUser);
      
      setMessage({ type: 'success', text: 'Profile updated successfully.' });
      setIsEditing(false);
      setSelectedFile(null);
      setPreview(null);
      
    } catch (error) {
      console.error("Update failed", error);
      setMessage({ type: 'error', text: 'Failed to update profile.' });
    } finally {
      setIsLoading(false);
    }
  };


  if (!user) return null;

  return (
          <div className="pt-8">
            <NavBar />
    <div className="max-w-4xl mx-auto p-6 animate-fade-in">
      <div className="w-full max-w-md mx-auto">
        <div className="bg-white rounded-4xl shadow-apple-xl p-8 border border-gray-100">
          <div className="flex flex-col items-center">
            <div className="relative">
              <img
                src={user.dp || `https://ui-avatars.com/api/?name=${user.full_name}&background=random`}
                alt="Profile"
                className="w-24 h-24 rounded-full object-cover"
              />
            </div>
            
            <button 
              onClick={() => setIsEditing(true)}
              className="mt-4 p-2 text-gray-400 hover:text-blue-500 hover:bg-blue-50 rounded-full transition-all"
              title="Edit Profile"
            >
              <Pencil size={20} />
            </button>

            <h2 className="text-xl font-semibold text-gray-900 mt-4">{user.full_name}</h2>
            <p className="text-gray-500">{user.email}</p>
          </div>
        </div>
      </div>

      <Modal isOpen={isEditing} onClose={() => setIsEditing(false)} title="Edit Profile">
        <form onSubmit={handleUpdate} className="space-y-6">
          <div className="flex flex-col items-center mb-6">
            <div className="relative">
              <img
                src={preview || user.dp || `https://ui-avatars.com/api/?name=${user.full_name}&background=random`}
                alt="Profile Preview"
                className="w-24 h-24 rounded-full object-cover"
              />
              <label htmlFor="dp-upload" className="absolute bottom-0 right-0 bg-blue-500 text-white rounded-full p-2 cursor-pointer hover:bg-blue-600 transition-colors shadow-md">
                <Upload size={16} />
                <input id="dp-upload" type="file" className="hidden" onChange={handleFileChange} accept="image/png, image/jpeg" />
              </label>
            </div>
            <p className="text-sm text-gray-500 mt-2">Click camera icon to change</p>
          </div>

          <div>
            <label className="block text-sm font-semibold text-gray-700 mb-2">Full Name</label>
            <div className="relative">
              <User className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400" size={20} />
              <input 
                type="text" 
                value={name}
                onChange={e => setName(e.target.value)}
                className="w-full pl-12 pr-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
              />
            </div>
          </div>

          <div>
            <label className="block text-sm font-semibold text-gray-700 mb-2">Email Address</label>
            <div className="relative">
              <Mail className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-400" size={20} />
              <input 
                type="email" 
                value={email}
                onChange={e => setEmail(e.target.value)}
                className="w-full pl-12 pr-4 py-3 rounded-xl bg-gray-50 border-transparent focus:bg-white focus:ring-2 focus:ring-blue-500 transition-all outline-none"
              />
            </div>
          </div>

          {message && (
            <div className={`p-4 rounded-xl text-sm font-medium ${message.type === 'success' ? 'bg-green-50 text-green-700' : 'bg-red-50 text-red-600'}`}>
              {message.text}
            </div>
          )}

          <div className="flex justify-end space-x-4 pt-4">
            <Button type="button" variant="ghost" onClick={() => setIsEditing(false)}>Cancel</Button>
            <Button type="submit" isLoading={isLoading}>
              <Save size={18} className="mr-2" />
              Save Changes
            </Button>
          </div>
        </form>
      </Modal>
      
      <div className="mt-8">
        <h2 className="text-2xl font-bold text-center text-gray-900 mb-6">Your Uploads</h2>
        <SubjectList
          subjects={subjects}
          isLoading={subjectsLoading}
          onSubjectClick={(subject) => navigate(`/subject/${subject.id}`)}
        />
        
      </div>
    </div>  
    </div>

  );
};

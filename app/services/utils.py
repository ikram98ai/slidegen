from typing import List
from fastapi import HTTPException
from langchain_core.documents import Document
from langchain_community.document_loaders import PyMuPDFLoader, TextLoader, Docx2txtLoader



async def get_file_contents(s3_url:str) -> List[Document]:
    try:
        if s3_url.lower().endswith(".pdf"):
            loader = PyMuPDFLoader(file_path=s3_url)
        elif s3_url.lower().endswith(".txt"):
            loader = TextLoader(file_path=s3_url)
        elif s3_url.lower().endswith(".docx") or s3_url.endswith(".doc"):
            loader = Docx2txtLoader(file_path=s3_url)
        else:
            raise HTTPException(400, f"Unsupported file type: {s3_url}")
        
        documents = loader.aload()
        return documents
    
    except Exception as e:
        print(f"Error loading PDF file {s3_url}: {str(e)}")
        raise HTTPException(400, f"Error loading PDF file {s3_url}: {str(e)}")

import struct
import os
import numpy as np
import streamlit as st
import matplotlib.pyplot as plt

# Fonction pour lire les nombres premiers depuis le fichier binaire
def lire_nombres_premiers(fichier):
    if not os.path.exists(fichier):
        st.error(f"Fichier '{fichier}' introuvable.")
        return np.array([])
    with open(fichier, "rb") as f:
        data = f.read()
        count = len(data) // 4  # 4 octets par entier
        return np.array(struct.unpack(f'<{count}I', data), dtype=np.uint64)

# Lecture des nombres premiers
data_file = "nombres_premiers.bin"
nombres_premiers = lire_nombres_premiers(data_file)

st.title("Analyse des Nombres Premiers")

if nombres_premiers.size == 0:
    st.stop()

st.write(f"Nombre total de nombres premiers chargés : {len(nombres_premiers):,}")

# Sélection de l'intervalle
defaut_min = int(nombres_premiers[0])
defaut_max = int(nombres_premiers[-1])
intervalle = st.slider(
    "Choisissez l'intervalle d'analyse",
    min_value=defaut_min,
    max_value=defaut_max,
    value=(defaut_min, min(defaut_max, defaut_min + 100_000)),
    step=1
)

borne_min, borne_max = intervalle
selection = nombres_premiers[(nombres_premiers >= borne_min) & (nombres_premiers <= borne_max)]

st.write(f"Nombres premiers dans l'intervalle [{borne_min:,} ; {borne_max:,}] : {len(selection):,}")

# Choix du type de graphe
graphe = st.selectbox(
    "Type de graphe",
    ["Histogramme (répartition)", "Espacement entre premiers"]
)

if graphe == "Histogramme (répartition)":
    nb_bins = st.slider("Nombre de classes (bins)", 10, 200, 50)
    fig, ax = plt.subplots()
    ax.hist(selection, bins=nb_bins, color='royalblue', edgecolor='black')
    ax.set_title("Répartition des nombres premiers")
    ax.set_xlabel("Valeur")
    ax.set_ylabel("Nombre de premiers")
    st.pyplot(fig)
elif graphe == "Espacement entre premiers":
    if len(selection) < 2:
        st.warning("Intervalle trop petit pour calculer les espacements.")
    else:
        espacements = np.diff(selection)
        fig, ax = plt.subplots()
        ax.hist(espacements, bins=30, color='orange', edgecolor='black')
        ax.set_title("Distribution des espacements entre nombres premiers")
        ax.set_xlabel("Espacement")
        ax.set_ylabel("Fréquence")
        st.pyplot(fig)

st.info("Lancez ce dashboard avec :\n\n    streamlit run analyse.py\n\nDépendances : streamlit, matplotlib, numpy") 
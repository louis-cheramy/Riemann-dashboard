import numpy as np
import plotly.graph_objects as go

# Partie imaginaire des premiers zéros non triviaux connus (sur Re(s) = 1/2)
ZEROS_NON_TRIVIAUX_IM = [
    14.134725, 21.022040, 25.010858, 30.424876, 32.935062,
    37.586178, 40.918719, 43.327073, 48.005150, 49.773832,
    52.970321, 56.446247, 59.347044, 60.831780, 65.112544,
    67.079811, 69.546402, 72.067158, 75.704690, 77.144840,
    79.337375, 82.910381, 84.735493, 87.425275, 88.809111,
    92.491899, 94.651344, 95.870634, 98.831194, 101.317851,
    103.725538, 105.446623, 107.168611, 111.029536, 111.874659,
    114.320221, 116.226680, 118.790782, 121.370125, 122.946829,
    124.256818, 127.516684, 129.578704, 131.087688, 133.497737,
    134.756510, 138.116042, 139.736209, 141.123707, 143.111846,
]


def zeros_triviaux(nb=20):
    return np.arange(-2, -2 - 2 * nb, -2, dtype=float)


def zeros_non_triviaux(im_min=-np.inf, im_max=np.inf):
    filtered = [y for y in ZEROS_NON_TRIVIAUX_IM if im_min <= y <= im_max]
    return [(0.5, y, i + 1) for i, y in enumerate(filtered)]


def figure_2d(im_min, im_max, nb_triviaux=20, animer=False):
    triviaux = zeros_triviaux(nb_triviaux)
    non_triviaux = zeros_non_triviaux(im_min, im_max)

    fig = go.Figure()

    fig.add_trace(go.Scatter(
        x=[0.5, 0.5],
        y=[im_min, im_max],
        mode="lines",
        line=dict(color="rgba(120,120,120,0.5)", width=2, dash="dash"),
        name="Droite critique Re(s)=1/2",
        hoverinfo="skip",
    ))

    fig.add_trace(go.Scatter(
        x=triviaux,
        y=np.zeros_like(triviaux),
        mode="markers",
        marker=dict(size=12, color="#2563eb", symbol="square", line=dict(width=1, color="black")),
        name="Zéros triviaux",
        text=[f"s = {int(x)}" for x in triviaux],
        hovertemplate="Trivial<br>Re(s)=%{x}<br>Im(s)=0<extra></extra>",
    ))

    if non_triviaux:
        re_vals = [z[0] for z in non_triviaux]
        im_vals = [z[1] for z in non_triviaux]
        indices = [z[2] for z in non_triviaux]
        fig.add_trace(go.Scatter(
            x=re_vals,
            y=im_vals,
            mode="markers+lines",
            marker=dict(
                size=10,
                color=indices,
                colorscale="Plasma",
                showscale=True,
                colorbar=dict(title="Rang n"),
                line=dict(width=1, color="black"),
            ),
            line=dict(color="rgba(220,38,38,0.35)", width=1),
            name="Zéros non triviaux",
            text=[f"n={n}, Im(s)={y:.3f}" for n, y in zip(indices, im_vals)],
            hovertemplate="Non trivial n°%{text}<br>Re(s)=0.5<br>Im(s)=%{y:.6f}<extra></extra>",
        ))

    if animer and non_triviaux:
        frames = []
        for k in range(1, len(non_triviaux) + 1):
            subset = non_triviaux[:k]
            frames.append(go.Frame(
                data=[
                    go.Scatter(x=[0.5, 0.5], y=[im_min, im_max], mode="lines",
                               line=dict(color="rgba(120,120,120,0.5)", width=2, dash="dash")),
                    go.Scatter(x=triviaux, y=np.zeros_like(triviaux), mode="markers",
                               marker=dict(size=12, color="#2563eb", symbol="square")),
                    go.Scatter(
                        x=[z[0] for z in subset],
                        y=[z[1] for z in subset],
                        mode="markers+lines",
                        marker=dict(size=10, color=[z[2] for z in subset], colorscale="Plasma"),
                    ),
                ],
                name=str(k),
            ))
        fig.frames = frames
        fig.update_layout(
            updatemenus=[{
                "type": "buttons",
                "showactive": False,
                "buttons": [
                    {"label": "▶ Animer", "method": "animate", "args": [None, {
                        "frame": {"duration": 400, "redraw": True},
                        "fromcurrent": True,
                        "transition": {"duration": 200},
                    }]},
                    {"label": "⏸ Pause", "method": "animate", "args": [[None], {
                        "frame": {"duration": 0, "redraw": False},
                        "mode": "immediate",
                    }]},
                ],
            }],
        )

    y_min = min(im_min, -5)
    y_max = max(im_max, 5)
    fig.update_layout(
        title="Zéros de ζ(s) dans le plan complexe (Re(s), Im(s))",
        xaxis=dict(title="Re(s)", range=[-42, 2], zeroline=True, gridcolor="rgba(0,0,0,0.08)"),
        yaxis=dict(title="Im(s)", range=[y_min, y_max], zeroline=True, gridcolor="rgba(0,0,0,0.08)"),
        template="plotly_white",
        height=550,
        legend=dict(orientation="h", yanchor="bottom", y=1.02),
        hovermode="closest",
    )
    return fig


def figure_3d(im_min, im_max, nb_triviaux=20):
    triviaux = zeros_triviaux(nb_triviaux)
    non_triviaux = zeros_non_triviaux(im_min, im_max)

    fig = go.Figure()

    fig.add_trace(go.Scatter3d(
        x=triviaux,
        y=np.zeros_like(triviaux),
        z=np.zeros_like(triviaux),
        mode="markers",
        marker=dict(size=6, color="#2563eb", symbol="square"),
        name="Zéros triviaux",
        text=[f"s = {int(x)}" for x in triviaux],
        hovertemplate="Trivial<br>Re(s)=%{x}<br>Im(s)=0<br>Indice n=0<extra></extra>",
    ))

    if non_triviaux:
        re_vals = [z[0] for z in non_triviaux]
        im_vals = [z[1] for z in non_triviaux]
        indices = [z[2] for z in non_triviaux]
        fig.add_trace(go.Scatter3d(
            x=re_vals,
            y=im_vals,
            z=indices,
            mode="markers+lines",
            marker=dict(
                size=5,
                color=indices,
                colorscale="Turbo",
                showscale=True,
                colorbar=dict(title="Rang n"),
            ),
            line=dict(color="rgba(220,38,38,0.5)", width=3),
            name="Zéros non triviaux",
            text=[f"n={n}" for n in indices],
            hovertemplate="Non trivial n°%{text}<br>Re(s)=0.5<br>Im(s)=%{y:.6f}<br>Rang=%{z}<extra></extra>",
        ))

    fig.add_trace(go.Scatter3d(
        x=[0.5, 0.5],
        y=[im_min, im_max],
        z=[0, max(len(non_triviaux), 1)],
        mode="lines",
        line=dict(color="gray", width=4, dash="dash"),
        name="Droite critique (3D)",
        hoverinfo="skip",
    ))

    fig.update_layout(
        title="Zéros de ζ(s) en 3D : Re(s), Im(s), rang du zéro",
        scene=dict(
            xaxis_title="Re(s)",
            yaxis_title="Im(s)",
            zaxis_title="Rang n",
            xaxis=dict(range=[-42, 2]),
            yaxis=dict(range=[im_min, im_max]),
            zaxis=dict(range=[0, max(len(non_triviaux), 5)]),
            bgcolor="rgb(247,247,250)",
        ),
        template="plotly_white",
        height=650,
        legend=dict(orientation="h", yanchor="bottom", y=1.02),
    )
    return fig
